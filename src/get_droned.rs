use flexi_logger::{Age, Cleanup, Criterion::Age as AgeCriterion, FileSpec, Logger, Naming};
use std::error::Error;

/// Initialize a global logger for the GetDroned drone.
/// You can initialize the logger in your network initializer or main function using this function.
/// but you can create your own logger in your code and use all the log of the GetDroned drone.
pub fn init_logger() -> Result<(), Box<dyn Error>> {
    Logger::try_with_str("info")
        .unwrap()
        .log_to_file(
            FileSpec::default()
                .directory("logs")
                .basename("")
                .suffix("log"),
        )
        .rotate(
            AgeCriterion(Age::Day),
            Naming::Timestamps,
            Cleanup::KeepLogFiles(10),
        )
        .format(|writer, now, record| {
            write!(
                writer,
                "{} [{}] [{}] {}",
                now.format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.target(),
                record.args()
            )
        })
        .start()
        .map(|_| ())
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

/// Represents a drone in the simulation.
///
/// A drone has a unique identifier, a packet drop rate, a list of neighboring drones,
/// and handles sending and receiving messages within the simulation.
#[derive(Debug)]
pub struct GetDroned {
    /// Unique identifier for the drone.
    id: NodeId,
    /// Probability of dropping a packet (0.00 to 1.00).
    packet_drop_rate: f32,
    /// The end where this drone receives messages from other nodes.
    receiver: Receiver<Packet>,
    /// The vector of all the neighbor ends where the drone can send messages.
    packet_senders: HashMap<NodeId, Sender<Packet>>,
    /// Sender to send events to the simulation controller.
    event_sender: Sender<DroneEvent>,
    /// Receiver to listen for commands from the simulation controller.
    command_receiver: Receiver<DroneCommand>,

    is_crashed: bool,

    received_floods: HashSet<(NodeId, u64)>,
}

impl Drone for GetDroned {
    /// Creates a new instance of `Drone`.
    ///
    /// ### Parameters
    /// - `id`: Unique identifier for the drone.
    /// - `packet_drop_rate`: Probability of dropping a packet.
    /// - `neighbors`: A vector of IDs representing neighboring drones.
    ///
    /// ### Returns
    /// A new `Drone` instance.
    fn new(
        id: NodeId,
        controller_send: Sender<DroneEvent>,
        controller_recv: Receiver<DroneCommand>,
        packet_recv: Receiver<Packet>,
        packet_send: HashMap<NodeId, Sender<Packet>>,
        pdr: f32,
    ) -> Self {
        info!(
            "Initializing Drone {}: packet_drop_rate={}, neighbors={:?}",
            id,
            pdr,
            packet_send.keys()
        );

        GetDroned {
            id,
            packet_drop_rate: pdr,
            receiver: packet_recv,
            packet_senders: packet_send,
            event_sender: controller_send,
            command_receiver: controller_recv,
            is_crashed: false,
            received_floods: HashSet::new(),
        }
    }

    /// Starts the drone execution loop.
    ///
    /// The drone will listen for incoming packets and commands. If a crash command is received, it stops execution.

    /// Starts the drone execution loop.
    ///
    /// The drone will listen for incoming packets and commands. If a crash command is received, it stops execution.
    fn run(&mut self) {
        info!("Drone {} started execution.", self.id);

        info!("Drone {} started execution.", self.id);

        loop {
            select_biased! {
                recv(self.command_receiver) -> command => {
                    match command {
                        Ok(command) => {
                            info!("Drone {} received a command: {:?}", self.id, command);
                            self.process_command(command);
                        },
                        Err(e) => warn!("Drone {} failed to receive a command: {:?}", self.id, e),
                    }
                },
                recv(self.receiver) -> packet => {
                    match packet {
                        Ok(packet) => {
                            info!("Drone {} received a packet: {:?}", self.id, packet);
                            self.process_packet(packet);
                        },
                        Err(e) => {
                            if self.is_crashed {
                                info!("Drone {} finished execution.", self.id);
                                return;
                            } else {
                                warn!("Drone {} failed to receive a packet: {:?}", self.id, e);
                            }
                        },
                    }
                },
            }
        }

        info!("Drone {} finished execution.", self.id);
    }
}

// * No function should be public (you can use only run and new functions from external)
impl GetDroned {
    /// Adds a neighboring sender to the drone's list of known neighbors.
    ///
    /// # Parameters
    /// - `id`: The unique ID of the neighboring node.
    /// - `sender`: The communication channel (`Sender<Packet>`) to send packets to the neighbor.
    fn add_neighbor_sender(&mut self, id: u8, sender: Sender<Packet>) {
        self.packet_senders.insert(id, sender);
    }

    /// Removes a neighboring sender from the drone's list of known neighbors.
    ///
    /// # Parameters
    /// - `id`: The unique ID of the neighboring node to be removed.
    fn remove_neighbor_sender(&mut self, id: NodeId) {
        self.packet_senders.remove(&id);
    }

    /// Sends a packet to a specific neighboring node.
    ///
    /// ### Parameters
    /// - `p`: The packet to be sent.
    /// - `dest_id`: The ID of the neighboring node to which the packet will be sent.
    ///
    /// ### Returns
    /// - `Ok(())` if the packet was sent successfully,
    /// - `Err(String)` if there was an error sending the packet.
    fn send_packet(&self, mut p: Packet) {
        let original_packet = p.clone();
        if let Some(next_hop) = p.routing_header.next_hop() {
            p.routing_header.hop_index += 1;
            if let Some(sender) = self.packet_senders.get(&next_hop) {
                match sender.send(p.clone()) {
                    Ok(_) => self.send_event(DroneEvent::PacketSent(p.clone())),
                    Err(_) => match p.clone().pack_type {
                        PacketType::FloodRequest(_flood_request) => self
                            .send_nack(original_packet.clone(), NackType::ErrorInRouting(self.id)),
                        PacketType::MsgFragment(_fragment) => self
                            .send_nack(original_packet.clone(), NackType::ErrorInRouting(self.id)),
                        _ => self.send_event(DroneEvent::ControllerShortcut(p.clone())),
                    },
                }
            }
        } else {
            self.send_event(DroneEvent::ControllerShortcut(p.clone()));
        }
    }

    /// Creates and sends a NACK packet to notify the sender of an error or specific event.
    /// Used to signal issues such as unexpected recipients, routing errors, or dropped packets.
    ///
    /// # Parameters
    /// - `packet`: The original packet that caused the issue.
    /// - `nack_type`: The type of error or event that occurred.
    fn send_nack(&self, mut packet: Packet, nack_type: NackType) {
        let nack = Nack {
            fragment_index: packet.get_fragment_index(),
            nack_type,
        };
        match nack_type {
            NackType::UnexpectedRecipient(_) => {
                for i in 0..packet.routing_header.hops.len() {
                    if self
                        .packet_senders
                        .contains_key(&packet.routing_header.hops[i])
                    {
                        packet.routing_header.hop_index = i + 1;
                        break;
                    }
                }
            }
            _ => {}
        }
        if let Some(routing_header) = packet
            .routing_header
            .sub_route(..packet.routing_header.hop_index + 1)
        {
            self.send_packet(Packet::new_nack(
                routing_header.get_reversed(),
                packet.session_id,
                nack,
            ));
        }
    }

    fn send_flood_request(&self, mut packet: Packet, received_from: NodeId) {
        for neighbor in self.packet_senders.clone() {
            if neighbor.0 != received_from {
                match neighbor.1.send(packet.clone()) {
                    Ok(_) => self.send_event(DroneEvent::PacketSent(packet.clone())),
                    Err(_) => {}
                }
            }
        }
    }

    /// Validates whether the received packet is correctly addressed and ready for processing.
    /// If the packet is invalid, sends an appropriate NACK to notify the sender.
    ///
    /// # Parameters
    /// - `packet`: The packet to validate.
    ///
    /// # Returns
    /// - `true` if the packet is valid and can proceed to processing.
    /// - `false` if the packet is invalid, and a NACK has been sent.
    fn validate_packet(&self, mut packet: Packet) -> Result<(), NackType> {
        if packet.routing_header.hops[packet.routing_header.hop_index] != self.id {
            return Err(NackType::UnexpectedRecipient(self.id));
        }
        packet.routing_header.hop_index += 1;
        if packet.routing_header.hop_index == packet.routing_header.hops.len() {
            return Err(NackType::DestinationIsDrone);
        }
        let next_hop = packet.routing_header.hops[packet.routing_header.hop_index];
        if !self.packet_senders.contains_key(&next_hop) {
            return Err(NackType::ErrorInRouting(next_hop));
        }
        Ok(())
    }

    /// Processes a received packet by determining its type and delegating its handling.
    /// First validates the packet, then handles it according to its specific type (e.g., message fragment or flood request).
    ///
    /// # Parameters
    /// - `packet`: The packet to process.
    fn process_packet(&mut self, packet: Packet) {
        match packet.clone().pack_type {
            PacketType::MsgFragment(_fragment) => match self.validate_packet(packet.clone()) {
                Ok(()) => self.process_fragment(packet.clone()),
                Err(nack_type) => self.send_nack(packet, nack_type),
            },
            PacketType::FloodRequest(flood_request) => {
                self.process_flood_request(packet, flood_request)
            }
            //Ack, Nack or FloodResponse
            _ => match self.validate_packet(packet.clone()) {
                Ok(()) => self.send_packet(packet.clone()),
                Err(_) => self.send_event(DroneEvent::ControllerShortcut(packet)),
            },
        }
    }

    /// Handles a message fragment by forwarding it to the next hop.
    /// Simulates packet drop based on the drone's packet drop rate, sending a NACK if the packet is dropped.
    ///
    /// # Parameters
    /// - `packet`: The message fragment to process.
    /// - `next_hop`: The ID of the next node in the routing path.
    fn process_fragment(&self, packet: Packet) {
        if self.is_crashed {
            self.send_nack(packet.clone(), NackType::ErrorInRouting(self.id));
            return;
        }
        if self.packet_drop_rate > 0.0 && rand::random::<f32>() < self.packet_drop_rate {
            self.send_nack(packet.clone(), NackType::Dropped);
            self.send_event(DroneEvent::PacketDropped(packet.clone()));
            return;
        }
        self.send_packet(packet)
    }

    /// Processes a flood request packet by determining the appropriate action based on the flood path
    /// and the drone's neighbors. The drone can either generate a response or forward the request.
    ///
    /// # Parameters
    /// - `packet`: The flood request packet being processed.
    /// - `flood_request`: The flood request data extracted from the packet.
    ///
    /// # Behavior
    /// - If the drone is already part of the flood path (`path_trace`), or it has no neighbors
    ///   other than the sender, it generates a response and sends it to the sender.
    /// - Otherwise, the drone forwards the flood request to all neighbors except the sender.
    fn process_flood_request(&mut self, mut packet: Packet, mut flood_request: FloodRequest) {
        let sender_id = match flood_request.path_trace.last() {
            Some((id, _)) => id.clone(),
            None => flood_request.initiator_id.clone(),
        };

        flood_request.increment(self.id, NodeType::Drone);

        if self
            .received_floods
            .contains(&(flood_request.initiator_id, flood_request.flood_id))
            || self.packet_senders.len() == 1
        {
            let response = flood_request.generate_response(packet.session_id);
            self.send_packet(response);
        } else {
            self.received_floods
                .insert((flood_request.initiator_id, flood_request.flood_id));
            packet.pack_type = PacketType::FloodRequest(flood_request);
            self.send_flood_request(packet.clone(), sender_id);
        }
    }

    /// Processes a command sent to the drone, modifying its state or behavior accordingly.
    ///
    /// # Parameters
    /// - `command`: A `DroneCommand` enum representing the action to be executed.
    ///
    /// # Supported Commands
    /// - `DroneCommand::AddSender(id, sender)`: Adds a neighboring sender to the drone's network.
    /// - `DroneCommand::Crash`: Marks the drone as crashed, disabling its functionality.
    /// - `DroneCommand::SetPacketDropRate(pdr)`: Sets the packet drop rate to simulate unreliable communication.
    /// - `DroneCommand::RemoveSender(id)`: Removes a neighboring sender from the drone's network.
    fn process_command(&mut self, command: DroneCommand) {
        match command {
            DroneCommand::AddSender(id, sender) => {
                self.add_neighbor_sender(id, sender);
            }
            DroneCommand::Crash => {
                self.is_crashed = true;
            }
            DroneCommand::SetPacketDropRate(pdr) => {
                self.packet_drop_rate = pdr;
            }
            DroneCommand::RemoveSender(id) => {
                self.remove_neighbor_sender(id);
            }
        }
    }

    /// Sends an event to Simulation Controller.
    ///
    /// # Parameters
    /// - `event`: A `DroneEvent` representing the event to be sent (e.g., packet sent, dropped).
    ///
    /// # Behavior
    /// - Attempts to send the event via the `event_sender` channel.
    /// - If the sending operation fails (e.g., the channel is closed), logs an error message with the failure reason.
    ///
    /// # Notes
    /// - This method ensures that events are dispatched asynchronously, allowing the drone to continue its operations.
    fn send_event(&self, event: DroneEvent) {
        match self.event_sender.send(event) {
            Ok(_) => (),
            Err(e) => println!("Failed to send event: {}", e),
        }
    }
}

impl Display for GetDroned {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "GetDroned {{ id: {}, packet_drop_rate: {}, packet_senders: {:?} }}",
            self.id, self.packet_drop_rate, self.packet_senders
        )
    }
}
