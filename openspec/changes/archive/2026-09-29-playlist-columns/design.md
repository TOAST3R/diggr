## Context

Rows are drawn by `playlist_section` as one string (`display_name`) plus a right-aligned duration or status. Reordering exists (`move_entry`, drag and drop). A crate's order drives play order, saving, export and the dig expansion's fetch order (from the playing entry onwards). After `dig-titles`, entries have `bpm`; after `player-layout`, the playlist can be any width.

## Goals / Non-Goals

**Goals:**
- Use the width for aligned, scannable columns.
- One-click sorting that behaves like Winamp's: it changes the crate's real order.

**Non-Goals:**
- A live, auto-resorting view.
- Multi-key sorts.
- Undo for a sort. A stable sort and an unchanged play position limit the harm; undo can come later, for all reorders at once.
- Filtering.

## Decisions

**1. A one-off sort that reorders the crate (the user's choice).**
`Playlist::sort_by(Field, Dir)` is a stable sort of the entries, followed by the same `mark_crate` path as a drag reorder. The playing entry keeps playing, and "next" follows the new order (the queue is rebuilt).

Alternative: a view sort over an unchanged order. Rejected, because BPMs arriving later would make entries jump, and "next" would disagree with what's on screen.

**2. Unknown values go last, whichever the direction.**
In a fresh dig crate most BPMs are unknown, and putting them first when sorting descending would bury the useful rows.

**3. Sort keys.**
| Column | Key |
|---|---|
| Cat# | natural order (`LT-2` < `LT-10`), case-insensitive |
| Artist, Title | case-insensitive, leading "The " not ignored (keep it simple) |
| BPM | the folded whole BPM |
| Side | natural order (`A2` < `A10` < `B1`) |
| Year | number |
| For sale | lowest price in cents; entries with "none for sale" after priced ones and before unknown ones |
| Time | duration |
| # | not sortable (it *is* the order) |

**4. Entries that arrive after a sort are not re-sorted.**
- Dig replacement puts clips where their listed entry was, so it keeps the sorted position.
- Enqueued entries go at the end.
- The header's sort arrow is cleared by any manual reorder, and by an add that lands out of order.

**5. The column-mode threshold is 480 points (at 1×).**
Below it the columns would be too narrow to help. The threshold is a constant, not a setting.

**6. Column widths are stored as fractions of the free width, with minimums.**
When the playlist is resized, the columns scale instead of overflowing. Title takes the remaining space.

## Risks / Trade-offs

- [An accidental header click reorders a big crate with no undo] → the sort is stable and the playing position is kept. A later change can add undo for all reorders.
- [Sorting while a dig expansion runs changes its fetch order] → accepted. Details are still fetched from the playing entry onwards, in the new order.
- [The header costs one row of height] → shown only in column mode.
