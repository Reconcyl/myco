# Command List

The following commands are supported by the editor:

### `q`, `quit`

Leave the editor.

Currently there is no session saving mechanism.

### `l`, `list`

Display a list of all living organisms along with their IDs, which are used to select them. The ID of an organism will change during its lifetime as new organisms are introduced. If this list is too long to fit in the info box, it can be scrolled with `w` and `s`.

### `max`

Report the current organism limit.

### `set-max [MAX]`

Set a limit on the number of organisms. At the moment, if there are too many organisms then a random one will be killed. In the future, some other heuristic may be used.

If no argument is passed, the limit will be disabled.

### `lifespan`

Report the maximum number of cycles an organism can live. This is 100 by default.

### `set-lifespan [MAX]`

Limit the lifespan of organisms to `MAX` cycles. If no argument is passed, organisms will be permitted to live for arbitrary amounts of time. Note that this only applies to organisms created after this command is run.

### `speed [SPEED]`

Accept an argument and set the execution rate to `SPEED` milliseconds per cycle. If no argument is passed, report the current speed.

### `seed`

Report the RNG seed. If this was not passed by the command line, it will be randomly generated.

### `source FILE`

Run the commands given by the lines of `FILE`. Blank commands and commands starting with `#` are ignored.

### `write-error-chance [CHANCE]`

Set the chance of a write error to 1 in `CHANCE`. If `CHANCE` is zero, then remove the possibility of write errors altogether. If no argument is passed, report the current chance of a write error.

### `cosmic-ray-rate [RATE]`

Set the frequency of cosmic rays to be `RATE` times per cycle. If no argument is passed, report the current frequency.

### `c [TIMES]`, `cycle [TIMES]`

Run `TIMES` cycles without displaying them. If no argument is passed, run a single cycle (equivalent to pressing space when paused).

### `p`, `pause`

Pause or unpause automatic execution. When paused, cycles can be executed by pressing space.

### `move DIR [TIMES]`

Move the cursor `TIMES` steps in the given direction. `DIR` should be `<`, `>`, `^`, or `v`. If `TIMES` is not passed, then move a single time.

### `write INS`

Write the given instruction symbol at the cursor.

### `| INS...`

This command takes any number of instructions as arguments and writes rightwards from the cursor, them move the cursor downwards. This instruction is intended to be used repeatedly in command files to embed patterns in the grid.

### `byte BYTE`

Like `write`, but accept argument as a byte value instead of an instruction symbol. This is only useful if you need to write a no-op byte that isn't 1.

### `spawn`

Create a new organism at the cursor moving rightwards. The initial organism has `ax = bx = flag = r = 0`. The memory array is entirely zero and the pointer points to the first element of it.

### `dedup`

Go through the list of organisms and remove any identical ones (i.e. those in the same position, moving in the same direction, and having the same state). Because organism behavior is deterministic, it is impossible for such organisms to ever diverge.

This also has the effect of unfocusing the focused organism (though this may change in the future).

### `auto-dedup [RATE]`

Set auto-deduplication to run once every `RATE` cycles. If `RATE` is zero, disable auto-deduplication altogether. If no argument is passed, report the current rate of auto-deduplication.

### `f [ID]`, `focus [ID]`

Set focus to the organism whose id is currently `ID`. If no argument is passed, remove focus from any organism.

### `v`, `view`

Scroll the view window such that the focused organism is in the top-left corner.

### `ip DIR [TIMES]`

Shift the focused organism's instruction pointer in the given direction. `DIR` should be `<`, `>`, `^`, or `v`. If `TIMES` is not passed, then move a single time.

### `r INS...`, `run INS...`

Have the focused organism run each instruction without moving.

### `kill`

Delete the focused organism.