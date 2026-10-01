## MODIFIED Requirements

### Requirement: Local only
The bridge SHALL listen only on the loopback address (127.0.0.1), on port 47800 by default, which can be changed in Options ▸ Browser…. It SHALL start only after the window is interactive, so launch stays within 300 ms. If the port is taken, the app SHALL start normally without the bridge and say so in Options ▸ Browser….

#### Scenario: Not reachable from the network
- **WHEN** another computer on the same network connects to the player's computer on the bridge port
- **THEN** the connection is refused

#### Scenario: Port taken
- **WHEN** another program already uses port 47800
- **THEN** the app starts normally, and Options ▸ Browser… says the port is taken

### Requirement: Pairing
Options ▸ Browser… SHALL show a 6-digit pairing code that is valid for 2 minutes. A request presenting that code SHALL receive a new random key of at least 128 bits, and the code SHALL then stop working. After 5 wrong codes, every pairing attempt SHALL be refused for 1 minute. The app SHALL store only a hash of each key. Forget browsers SHALL revoke every key.

#### Scenario: Pair once
- **WHEN** the extension sends the code shown in Options ▸ Browser… within 2 minutes
- **THEN** it receives a key, the dialog says a browser was paired, and the same code no longer works

#### Scenario: Guessing is stopped
- **WHEN** 5 wrong codes are sent in a row
- **THEN** every pairing attempt is refused for 1 minute, even one with the right code
