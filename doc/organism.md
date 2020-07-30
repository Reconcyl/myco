Each organism has access to the following state:

- An instruction pointer (IP) pointing somewhere on the grid.
- A movement direction (`dir`) which can be up, down, left, or right.
- Two registers (`ax` and `bx`) which hold bytes.
- A general-purpose boolean control-flow flag (`f`).
- A cursor pointing somewhere on the grid.
- A selection radius (`r`) ranging from 0 to 10. Attempting to set `r` to a value out of this range will have no effect.
- A clipboard, which is a square of bytes with odd side length between 1 and 21.

Arithmetic involving byte values always wraps.

The following instructions are supported:

| **Name** | **Effect** |
| ---- | - |
| | **Life cycle** |
| `@@` | End execution. |
| `..` | Do nothing. |
| `##` | Do nothing, but cannot be moved onto by any organism's cursor. |
| `-=` | Create a new organism with exactly the same IP and state, except that the original's `f` is true and the clone's `f` is false. |
| `m=` | Create a new organism with exactly the same state, except that its IP is set to the same place as its cursor. |
| | **Data manipulation** |
| `0a` | `ax = 0` |
| `0b` | `bx = 0` |
| `ba` | `bx = ax` |
| `ab` | `ax = bx` |
| `::` | `tmp = ax; ax = bx; bx = tmp` |
| `a+` | `ax = ax + bx` |
| `b+` | `bx = ax + bx` |
| `a-` | `ax = -ax` |
| `b-` | `bx = -bx` |
| `+a` | `++ax` |
| `+b` | `++bx` |
| `-a` | `--ax` |
| `-b` | `--bx` |
| `a*` | `ax = ax * bx` |
| `b*` | `bx = ax * bx` |
| `aa` | `ax = ax * 2` |
| `bb` | `bx = bx * 2` |
| `a/` | `ax = ax / 2` (rounding down) |
| `b/` | `bx = bx / 2` (rounding down) |
| `a%` | `ax = ax % 2` |
| `b%` | `bx = bx % 2` |
| `a&` | `ax = ax & bx` |
| `b&` | `bx = ax & bx` |
| `a\|` | `ax = ax | bx` |
| `b\|` | `bx = ax | bx` |
| `a#` | `ax = ax ^ bx` |
| `b#` | `bx = ax ^ bx` |
| `a=` | `ax = (ax == bx)` |
| `b=` | `bx = (ax == bx)` |
| `a!` | `ax = (ax != bx)` |
| `b!` | `bx = (ax != bx)` |
| `a0` | `ax = (ax == 0)` |
| `b0` | `bx = (bx == 0)` |
| `a1` | `ax = (ax != 0)` |
| `b1` | `bx = (bx != 0)` |
| | **Control flow** |
| `.a` | Delay `ax` cycles. |
| `.b` | Delay `bx` cycles. |
| `!<` | `dir = <` |
| `!>` | `dir = >` |
| `!^` | `dir = ^` |
| `!v` | `dir = v` |
| `?<` | `if (f) { dir = < }` |
| `?>` | `if (f) { dir = > }` |
| `?^` | `if (f) { dir = ^ }` |
| `?v` | `if (f) { dir = v }` |
| `?@` | End execution if `f` is true.x3 |
| `!#` | Rotate `dir` 180 degrees. |
| `!\|` | Rotate `dir` 180 degrees if it is horizontal. |
| `!-` | Rotate `dir` 180 degrees if it is vertical. |
| `!/` | Rotate `dir`, mapping `>` to `^` and vice versa, and mapping `<` to `v` and vice versa. |
| `!\\` | Rotate `dir`, mapping `<` to `^` and vice versa, and mapping `>` to `v` and vice versa. |
| `((` | `f = true` (1) |
| `))` | `f = false` (0) |
| `(a` | `f = (ax == 0)` |
| `)a` | `f = (ax != 1)` |
| `(b` | `f = (bx == 0)` |
| `)b` | `f = (bx != 1)` |
| `(=` | `f = (ax == bx)` |
| `(!` | `f = (ax != bx)` |
| `)(` | `f = !f` |
| `a(` | `ax = f` |
| `b(` | `bx = f` |
| `#<` | Move the cursor left. |
| `#>` | Move the cursor right. |
| `#^` | Move the cursor up. |
| `#v` | Move the cursor down. |
| `a<` | Move the cursor left `ax` steps. |
| `a>` | Move the cursor right `ax` steps. |
| `a^` | Move the cursor up `ax` steps. |
| `av` | Move the cursor down `ax` steps. |
| `b<` | Move the cursor left `bx` steps. |
| `b>` | Move the cursor right `bx` steps. |
| `b^` | Move the cursor up `bx` steps. |
| `bv` | Move the cursor down `bx` steps. |
| `#0` | Set the cursor to the IP. |
| | **Cursor movement and selection** |
| `ra` | `r = ax` |
| `rb` | `r = bx` |
| `r0` | `r = 0` |
| `ar` | `ax = r` |
| `br` | `bx = r` |
| `r+` | `r = r + 1` |
| `r-` | `r = r - 1` |
| `ma` | Set the byte at the cursor to `ax`. |
| `mb` | Set the byte at the cursor to `bx`. |
| `am` | Set `ax` to the byte at the cursor. |
| `bm` | Set `bx` to the byte at the cursor. |
| `cm` | Copy the selection to the clipboard. |
| `mc` | Paste the clipboard at the cursor. |
