
## Ideas

Current setup.

To start the game, we pass ownership of the game struct to itself in the start fn. THis seems pretty necessary?

One question is, how will we update status?

Status mostly consists of Phase:
- Day/Night
- Day number
- votes

Not strictly necessary to have game run by a thread.....

Could pass commands in potentially. Handle a day, (events going out are still necessary). But status can be queried more easily.

No game thread. Just a `fn handle(&mut self, cmd: Command<U,S>) -> Phase<U>`. Discord handler can get back status after every handle, and write that (potentially after pulling off all events from queue)


### Paradigm

- Server receives discord bot http info.
- Passes info to game controller.
- Parses info into a controller access, or into a *Game Command*
- Calls Game Command on Game. Returns status. 
- Pulls from Event Channel and pushes out the appropriate responses
- Updates game status, pushes that.

No Game thread!

## TODO!
Make events push Players, not just Pidx.
