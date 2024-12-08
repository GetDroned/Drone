# GetDroned

GetDroned is a drone implementation that can send and receive messages, handle commands, and simulate packet drops. The main functionalities of GetDroned include:

## Features

- **Initialization**: Initialize a global logger for the GetDroned drone.
- **Drone Creation**: Create a new instance of a drone with a unique identifier, packet drop rate, and neighboring drones.
- **Packet Handling**: Send and receive packets, including message fragments and flood requests.
- **Command Processing**: Handle commands such as adding/removing neighbors, crashing the drone, and setting packet drop rates.
- **Event Logging**: Log events such as packet sent, packet dropped, and command received.

## Usage

To use GetDroned, you can initialize the logger in your network initializer or main function using the `init_logger` function. You can also just use your own log initialization function.

## Contact

For any modifications or support, we are available 24/7!
