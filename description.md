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

Chat history for each room is stored in hidden a directory created by the app under the home directory.

 - Communication architecture - 

 chatter users 2 protocol communication style. HTTP for server control, and TCP WebSocket for asynchronous server responses. Such architecture provide convienient separation of control and broadcast data flow. Reduces also amount of required code, combining best of both worlds - HTTP transactions and error notifications with WS agility. 

 - HTTP data flow: CLIENT -> SERVER, transaction result handling on app protocol layer
 - WS   data flow: SERVER -> CLIENT, no transaction result handling on app protocol layer (only TCP handshake) 

 Client uses HTTP to send messages, room joining/leaving, registration, client life notification service                    
 Server uses websockets to transfer messages to listening clients with room distingishing 