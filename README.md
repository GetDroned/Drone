# GetDroned 🚀

**GetDroned** is an implementation of a drone that simulates the behavior of nodes in a distributed network. Drones can send and receive messages, manage commands, and simulate packet drops.
Our team is characterized by multiculturalism, and working together has allowed us to merge different perspectives, improving the project both technically and creatively. We believe that this synergy is the key to the success of **GetDroned**.

## **Key Features**

- **Drone Creation**: Create a drone with a unique ID, a packet drop probability, and a network of neighbors.
- **Packet Management**: Sending, receiving, and validating packets, including message fragments and flooding requests.
- **Command Processing**: Execute commands to modify neighbors, crash the drone, or set the drop rate.
- **Event Logging**: Track significant events such as sent packets, lost packets, and received commands (optional `log` feature).

---

## **Importing GetDroned**

Add these dependencies to your `Cargo.toml`:

```toml
[dependencies]
crossbeam-channel = "0.5.13"
flexi_logger = "0.29.6"
log = "0.4"
wg_2024 = { git = "https://github.com/WGL-2024/WGL_repo_2024.git", features = ["serialize"] }
getdroned = { git = "https://github.com/GetDroned/Drone.git", features = ["log"] }
```

## **Usage**

### **Creating a Drone**

To create a drone:

```rust
let drone = GetDroned::new(
    id,                       // Unique drone ID
    controller_sender,        // Channel for sending events
    controller_receiver,      // Channel for receiving commands
    packet_receiver,          // Channel for receiving packets
    packet_senders,           // Map of channels for neighbors
    0.1,                      // Packet drop rate (0.0 - 1.0)
);
```

### **Running the Drone**

To run a drone:

```rust
drone.run();
```

---

## **Event Logging**

The logging feature uses **flexi_logger** to manage rotating files. The logger saves files in the `logs/` directory. Logged events include:

- Sent packets: `PacketSent`
- Dropped packets: `PacketDropped`
- Received commands: `CommandReceived`

Sample output:

```
2024-12-12 14:30:45 [INFO] [GetDroned] Drone 1 received a packet: ...
2024-12-12 14:31:10 [WARN] [GetDroned] Drone 2 failed to receive a packet: ...
```

---

## **Contributions**

For feature requests or bug reports, contact us on Telegram:

- **@ElementalAether**
- **@PinkArtemis**
- **@arthurbrnn**
- **@quentin_grn**
