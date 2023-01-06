# Tasks
Things to work on and tips on how to work on them.

## Controller

The Controller needs to be implemented (See README)

Controller should probably have its own thread, so we need a way to spawn it.

Also want a mspc::channel to get Commands in a thread-safe way

Can hold an instance of `Game`. Routes Game Actions there.
- Handles Errors

## Server

The Server needs to be implemented (See README)

## Saving

Design game saving method.

## Module Organization

Figure out:
- Re-exportations (what is exported in super modules?)
    - interface
    - core
    - etc
- pub (what can be made private in the first place?)

We can explicitly re-export some things to allow others to be private?

- core (Platform agnostic game logic and types)
    -  phase (for status)
    -  contracts (for status)
    -  player (for status?)
    -  interface
        -  command
        -  event
        -  error
    -  rules
        - rolegen
        - roleset
    - roles
- controller
    - interface
- server
