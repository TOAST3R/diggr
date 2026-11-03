Diggr for Windows
=================

Diggr is a player for digging records: Discogs, Bandcamp and Juno pages become crates of
previews to want, pass or buy.

Install
-------
There is nothing to install. Extract this folder anywhere (right-click the .zip, Extract All)
and open Diggr.exe. You can move or delete the folder later: your crates and settings are kept
in your user profile, not here.

First open: "Windows protected your PC"
----------------------------------------
Diggr.exe is signed by SignPath Foundation (right-click it, Properties, Digital Signatures),
so Windows 11 with Smart App Control on lets it run. Windows SmartScreen may still stop it
once, until enough people have downloaded it:

  1. In the blue window, click "More info" (the publisher shown is SignPath Foundation).
  2. Click "Run anyway".

It opens normally from then on, also offline. Diggr asks for no other permission.

Previews need yt-dlp
--------------------
Previews are downloaded with yt-dlp, a free downloader that Diggr doesn't ship on Windows.
Until it is installed, entries say "needs yt-dlp". Install it from
https://github.com/yt-dlp/yt-dlp/releases (yt-dlp.exe) into a folder that is on your PATH, or
put it anywhere and set its path in OPT > Discogs... . Diggr finds it within 30 seconds.

Where Diggr keeps its files
---------------------------
  settings, crates       %APPDATA%\Diggr\
  caches, previews       %LOCALAPPDATA%\Diggr\

Delete those two folders (and this one) to remove Diggr completely.

License
-------
Diggr is free software under the GNU General Public License, version 3 or later: see
LICENSE.txt in this folder. It comes with no warranty.

Tested on Windows 10 (64-bit). Windows 11 is supported, with Smart App Control on or off.
More: https://github.com/TOAST3R/diggr
Code signing: https://diggr.es/code-signing/
