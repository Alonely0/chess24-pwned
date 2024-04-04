# Where to find the rendered files

They're in process of being uploaded to the Internet Archivee, I'll update this when it's done.

# Chess24-pwned
This is a personal project with the objective of preserving and archiving the content of the Chess24 (paid or otherwise) from being lost due to the acquisition of said platform by Chess.com LLC. No intellectual property of the aforementioned entities is shipped in this repository, only clean-room reverse-engineered code that replicates their render engine to the best of my efforts and a scraper that downloads data the account holder had the permission to view. Thus, no law is being broken in either Spain (where I am based) or the U.S. (where they are), sue me. The pieces used are *Copyright (c) Colin M.L. Burnett* and distributted under the *[CC-BY-SA 3.0](https://creativecommons.org/licenses/by-sa/3.0/)*, and thus any video generated with these pieces will inherit said license.

# How to use
The default, production-mode of the tool (can be source-switched in `renderer/src/main.rs`) expects two arguments:
* **In-dir**: a path to a directory in the following internal structure: `course/chapter/{0.json, video.webm}` where these last two files represent the board data and the tutor video, respectively.
* **Out-dir**: a path to a directory where the rendered files will be saved and errors will be logged into, as unexpected issues are logged via both stderr and written logfiles.

By default, *H.264* will be used to encode the videos, although the FFmpeg options can be modified in `renderer/src/video.rs`. I'm only using it because it ran fast enough on my CPU to render it all in a few days, consider yourself encouraged to render them with AV1 if you have a GPU with good hardware support such as the Intel Arcs or most high-end NVIDIAs & AMDs.

# What's left to do
Not much. There's a very low priority *WONTFIX* in the interpreter that I couldn't be arsed to fix, and a nice-to-have would be to render the different game lines and its movements under the video, just like Chess24 did. I just don't have the energy to fix neither since they don't really affect the content consumption that much. If you really, really need either of them, I'll just do it, though.
