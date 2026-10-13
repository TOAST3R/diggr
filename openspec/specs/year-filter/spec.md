# year-filter Specification

## Purpose
Narrows any crate of records by the year Discogs gives each record, as diggers look by era: years picked in a grid in the filter panel (a click for one, a Shift-click for a run), combined with the other filters, remembered per crate.
## Requirements
### Requirement: Year filter
In any crate whose records have at least two different known years, the user SHALL be able to filter the crate by year. A record's year SHALL be the year Discogs gives for it: the pressing's year for a release, the master's for a master. A record without one has no known year. With no year picked, the filter SHALL show every entry. With years picked, an entry SHALL be shown when its record's year is one of them. An entry with no known year SHALL be shown only while the filter is off. The filter SHALL combine with the others as `record-filters` "Filters combine" says, and SHALL be offered, laid out and cleared like the other record filters: a YEARS button in the filter bar, lit and counting the picked years ("YEARS 3"), a double-click on it, Clear, the bar's × and `P` on a hidden playing entry turning it off.

#### Scenario: Some years
- **WHEN** a crate holds records from 1991, 1994, 1999 and 2008, and the user picks 1991 and 1999
- **THEN** only the 1991 and 1999 records are shown and played, and YEARS reads "YEARS 2"

#### Scenario: Unknown years
- **WHEN** a seller crate holds records from 1995 and 2010 and two records without a year, and the user picks 1995
- **THEN** only the 1995 record is shown

#### Scenario: Not enough years
- **WHEN** every record of the shown crate is from 1996, or none has a known year
- **THEN** the filter bar offers no YEARS button and the panel no YEAR tab

#### Scenario: Master entries
- **WHEN** a label crate lists a master whose Discogs year is 1993, and one of its 2016 reissues as a release
- **THEN** the master's entries have the year 1993 and the reissue's entries 2016

#### Scenario: Big crate
- **WHEN** the user picks a year in a grouped collection crate of 5,000 entries while a track plays
- **THEN** the list updates within 16 ms, and playback has zero underruns

### Requirement: Year grid
The filter panel's YEAR tab SHALL show a grid of every year from the crate's earliest known year to its latest, five a row, rows starting on a year ending in 0 or 5, each cell showing the year and its number of records ("1994", "12"). A year without records SHALL show "·" and SHALL NOT be clickable. A click on a year SHALL pick it, or unpick it when picked. A Shift-click SHALL pick every year with records from the year clicked before to this one, unpicking none. Picked years SHALL be shown selected. The tab SHALL end with Clear.

#### Scenario: Grid
- **WHEN** the YEAR tab is open in a crate with records from 1991 to 2003
- **THEN** the grid runs from 1990 to 2003 in rows of five, and 1990 and the years without records show "·" and can't be clicked

#### Scenario: Shift-click a run
- **WHEN** the user clicks 1991, then Shift-clicks 1997, in a crate with records from 1991, 1994 and 1997
- **THEN** 1991, 1994 and 1997 are picked

### Requirement: Year filter is remembered
The picked years SHALL be remembered per crate, across restarts. A picked year that no record of the crate has any more SHALL stop filtering. Crate files saved before this change SHALL load with no year picked.

#### Scenario: After a restart
- **WHEN** 1991 and 1994 are picked in a friend's crate and the app is restarted
- **THEN** showing that crate shows only its 1991 and 1994 records

#### Scenario: Year gone
- **WHEN** 1994 is the only picked year and refreshing the crate removes its 1994 records
- **THEN** every entry of the crate is shown

