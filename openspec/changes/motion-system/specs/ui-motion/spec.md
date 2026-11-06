# Spec Delta

## Purpose

How the player animates things appearing and disappearing, so the interface feels fluid while
every action still takes effect at once and an idle window costs nothing.

## ADDED Requirements

### Requirement: Menus and popups appear with motion
A context menu or popup SHALL fade in from transparent and rise 4 px into place over 120 ms
when it opens, and SHALL fade out over 80 ms when it closes.

#### Scenario: A context menu opens
- **WHEN** the user right-clicks a playlist row
- **THEN** its menu goes from transparent to fully drawn in 120 ms (±1 frame), rising 4 px, and stays still afterwards

#### Scenario: A menu closes
- **WHEN** the user clicks outside an open menu
- **THEN** it fades out over 80 ms (±1 frame) and is then gone

### Requirement: Dialogs appear with motion
A dialog shown inside the main window (Discogs, Browser, Download all tracks, the welcome setup,
the help panel, the record card) SHALL fade in and grow from 96% to full size about its centre over 180 ms when it
opens, and SHALL fade and shrink back over 120 ms when it closes.

#### Scenario: The help panel opens
- **WHEN** the user presses H
- **THEN** the help panel goes from 96% to 100% of its size and from transparent to opaque in 180 ms (±1 frame)

### Requirement: Panels slide in from their edge
A panel that drops from a control (the filters panel) SHALL slide 12 px in from the side of that
control and fade in over 240 ms when it opens, and SHALL slide back out over 160 ms when it
closes. The record card's slide to another record SHALL keep its timing.

#### Scenario: The filters panel opens
- **WHEN** the user clicks FILTERS
- **THEN** the panel slides 12 px to its place and fades in, in 240 ms (±1 frame)

### Requirement: Separate windows fade their content in
A separate window (EQ, spectrogram, insights) SHALL fade its content in from the window's
background over 180 ms when it opens.

#### Scenario: The EQ opens
- **WHEN** the user clicks the mini player's EQ icon
- **THEN** the EQ window's content goes from its background colour to fully drawn in 180 ms (±1 frame)

### Requirement: Motion never delays an action
An action SHALL take effect in the frame it is made, whatever is animating. Something that opens
SHALL take clicks and keys at its final place from its first frame. Something that closes SHALL
stop taking input in the frame it starts closing. No action SHALL wait for an animation.

#### Scenario: Clicking a menu item at once
- **WHEN** a context menu opens and the user clicks one of its items on the menu's first frame
- **THEN** the item's action runs in that frame

#### Scenario: A closing dialog takes no clicks
- **WHEN** a dialog is fading out and the user clicks where its button was
- **THEN** the click goes to what is underneath, not to the dialog

#### Scenario: Playback is untouched
- **WHEN** the user presses Space while a panel is sliding in
- **THEN** playback pauses in that frame, and the start, seek and underrun targets still hold

### Requirement: Repaint while moving, idle when still
While anything animates, the app SHALL repaint at the display's refresh rate. Once every
animation has settled and nothing plays, the app SHALL stop repainting until there is input.

#### Scenario: Smooth on a fast display
- **WHEN** a dialog opens on a 120 Hz display
- **THEN** its motion is drawn on every display refresh

#### Scenario: Idle after the motion
- **WHEN** nothing plays and the last animation settles
- **THEN** the app requests no further repaint until the next input

### Requirement: Reduce motion
Reduce motion, in the Options menu, SHALL turn all of the motion above off: things then appear and disappear in
a single frame. The setting SHALL persist, default to off, and a settings file without it SHALL
load as off.

#### Scenario: Turned on
- **WHEN** Reduce motion is on and the user opens a context menu
- **THEN** the menu is fully drawn in its first frame

#### Scenario: Old settings file
- **WHEN** the app starts with a settings file written before this setting existed
- **THEN** it starts with Reduce motion off and every other setting kept
