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
Diggr isn't signed with a paid certificate, so Windows SmartScreen stops it once:

  1. In the blue window, click "More info".
  2. Click "Run anyway".

It opens normally from then on, also offline. Diggr asks for no other permission.

Previews need yt-dlp
--------------------
Previews are downloaded with yt-dlp, a free downloader. The first time Diggr opens, and
whenever you play a preview without it, Diggr offers to install it: click Install. It
downloads about 56 MB (yt-dlp, and a Python and a JavaScript runtime that only Diggr uses),
checks each file, and keeps them with its caches. You can say Not now; entries then say
"needs yt-dlp", and clicking that icon asks again.

A yt-dlp.exe of your own still works: put it in a folder on your PATH, or set its path in
OPT > Discogs... . Diggr uses yours instead, and finds it within 30 seconds.

Where Diggr keeps its files
---------------------------
  settings, crates       %APPDATA%\Diggr\
  caches, previews       %LOCALAPPDATA%\Diggr\

Delete those two folders (and this one) to remove Diggr completely.

License
-------
Diggr is free software under the GNU General Public License, version 3 or later: see
LICENSE.txt in this folder. It comes with no warranty.

Tested on Windows 10 (64-bit). Windows 11 should work but isn't tested yet.
More: https://github.com/TOAST3R/diggr
