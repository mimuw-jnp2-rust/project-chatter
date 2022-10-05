# Chatter - a command-line chat with rooms

This app is a simple command-line chat written fully in Rust.
It allows multiple users to communicate with each other through multiple rooms.

Once a user opens the chat, they can:
- connect as an existing user,
- register (and connect) as a new user.
Then, they can:
- connect to an existing room,
- create (and connect to) a new room.

Joining and leaving a room results in a notification of the event being sent to remaining users.

The protocol used for client to server communication is HTTP. The server communicates with the client through a WebSocket connection. This model is alike to the one used by LinkedIn's chat service.

Chat history for each room is stored in hidden a directory created by the app under the home directory.