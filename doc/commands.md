# Command List

The following commands are supported by the editor:

## `q`, `quit`

Leave the editor.

Currently there is no session saving mechanism.

## `l`, `list`

Display a list of all living organisms along with their IDs, which are used to select them. The ID of an organism will change during its lifetime as new organisms are introduced. If this list is too long to fit in the info box, it can be scrolled with `w` and `s`.

## `max`

Report the current organism limit.

## `set-max [MAX]`

Accept an argument limiting the number of organisms. At the moment, if there are too many organisms then reproduction is simply impossible. In the future, organisms that reproduced least recently will be killed to make room for new ones.

If no argument is passed, the limit will be removed.

## `speed [SPEED]`

Accept an argument and set the execution rate to SPEED milliseconds per cycle. If no argument is passed, report the current speed.

## `seed`

Report the RNG seed. If this was not passed by the command line, it will be randomly generated.

## `source FILE`

Run the commands given by the lines of `FILE`. Blank commands and commands starting with `#` are ignored.

## `write-error-chance [CHANCE]`

Set the chance of a write error to 1 in `CHANCE`. If `CHANCE` is zero, then remove the possibility of write errors altogether. If no argument is passed, report the current chance of a write error.

## `c [TIMES]`, `cycle [TIMES]`

Run `TIMES` cycles without displaying them. If no argument is passed, run a single cycle (equivalent to pressing space when paused).

## `p`, `pause`

Pause or unpause automatic execution. When paused, cycles can be executed by pressing space.

## `move DIR [TIMES]`

Move the cursor `TIMES` steps in the given direction. `DIR` should be `<`, `>`, `^`, or `v`. If `TIMES` is not passed, then move a single time.

## `write INS`

Write the given instruction symbol at the cursor.

## `| INS...`

This command takes any number of instructions as arguments and writes rightwards from the cursor, them move the cursor downwards. This instruction is intended to be used repeatedly in command files to embed patterns in the grid.

## `byte BYTE`

Like `write`, but accept argument as a byte value instead of an instruction symbol. This is only useful if you need to write a no-op byte that isn't 1.

## `spawn`

Create a new organism at the cursor moving rightwards. The initial organism has `ax = bx = flag = r = 0`. The memory array is entirely zero and the pointer points to the first element of it.

## `dedup`

Go through the list of organisms and remove any identical ones (i.e. those in the same position, moving in the same direction, and having the same state). Because organism behavior is deterministic, it is impossible for such organisms to ever diverge.

This also has the effect of unfocusing the focused organism (though this may change in the future).

## `auto-dedup`

Report the frequency of auto-deduplication (how many cycles between every dedup).

## `set-auto-dedup [RATE]`

Set auto-deduplication to run once every `RATE` cycles. If no argument is passed, disable auto-deduplication altogether.

## `f ID`, `focus ID`

Set focus to the organism whose id is currently `ID`.

## `uf`, `unfocus`

Remove focus from any organism.

## `view`

Scroll the view window such that the focused organism is in the top-left corner.

## `ip DIR [TIMES]`

Shift the focused organism's instruction pointer in the given direction. `DIR` should be `<`, `>`, `^`, or `v`. If `TIMES` is not passed, then move a single time.

## `r INS...`, `run INS...`

Have the focused organism run each instruction without moving.

## `kill`

Delete the focused organism.