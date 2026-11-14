![SnapIntel Logo](https://github.com/Kr0wZ/SnapIntel/blob/master/assets/snapintel_logo.png?raw=true)

# SnapIntel

### What is it?

SnapIntel is a python tool providing you information about Snapchat users. You can list stories, curated highlights, spotlights, lenses and even download all of them!
The heatmap functionality allows you to generate a heatmap related to upload dates for specific categories.

### Updates

`05/08/2025`:
- You can now install **SnapIntel** to use it across your system with: `pip install .` once inside the main directory.

`09/10/2024`:
- Multi-threading has been implemented in the code for faster download (stories and bitmojis, see below)!
- A new option `-T/--threads` can now be used to specify the numbers of concurrent threads you wan to run (10 by default if not specified).
- Bitmojis for private users can now be downloaded and stored locally. We can also list them (it only shows the number of different bitmojis).

A big thanks to [@hatless1der](https://twitter.com/hatless1der) for its article about bitmojis: https://hatless1der.com/a-snapchat-osint-tip-viewing-bitmoji-changes/

`17/12/2023`:
The old tool (and its first version) was named **Snapchat_stories_dl** and was only built to download stories. But it has been replaced by this new 2.0 version with a lot more functionalities.


### Install

Clone this repository and jump in it:
```bash
git clone https://github.com/Kr0wZ/SnapIntel/
cd SnapIntel
```

You must have `python3` installed on your machine.
You can run this command to install `SnapIntel` and then use it from anywhere on your system:
```bash
pip install .

# Now you can use "snapintel" as a command (or "python3 -m snapintel")
```

To work on the code, install it in editable mode instead (`pip install -e .`): the command then runs the files in this folder, and `git pull` updates it without reinstalling.

Or run the following command to install the dependencies:
```bash
python3 -m pip install -r requirements.txt 
```

⚠️ Known issue ⚠️ <br>
If you have difficulties installing `numpy` try to downgrade your python3 version to `python3.8`


### Usage

Simply run the tool using python3 with `-h/--help` to show the help menu:
```bash
snapintel -h

#OR

python3 main.py -h
```
```
usage: snapintel [-h] [-u USERNAME] [-s] [-l [OPTIONS]] [-m] [-d [OPTIONS]] [-o DIRECTORY] [-t TIMEOUT] [-T THREADS] [--self-check [USERNAME]] [-V]

SnapIntel OSINT Tool - Made by KrowZ

options:
  -h, --help            show this help message and exit
  -u USERNAME, --username USERNAME
                        Username of account we want to download stories from
  -s, --stats           Only prints summary and statistics about the account
  -l [OPTIONS], --list [OPTIONS]
                        List the desired information. By default, lists only user information. 'a' = all, 's' = stories, 'u' = user, 'c' = curated highlights, 'p' = spotlights, 'l' = lenses, 'b' = bitmojis. Multiple options can be combined together: E.g: -l 'csl'
  -m, --heatmap         Generates a heatmap related to upload dates. Must be used with '-l/--list' option
  -d [OPTIONS], --download [OPTIONS]
                        Downloads specific videos posted by a user. By default, only downloads current stories. 'a' = all, 's' = stories, 'c' = curated highlights, 'p' = spotlights, 'l' = lenses, 'b' = bitmojis. Multiple options can be combined together: E.g: -d 'csl'
  -o DIRECTORY, --output DIRECTORY
                        Path where stories are stored. Default : current directory
  -t TIMEOUT, --timeout TIMEOUT
                        Requests timeout. Default : 30 seconds
  -T THREADS, --threads THREADS
                        Number of concurrent threads. Default : 10
  --self-check [USERNAME]
                        Check that Snapchat's page still matches config.json, then exit (1 if a field no longer resolves).
                        Without USERNAME it checks the reference accounts 'spectacles' and 'bitmoji', which cover every field.
                        Prints each field's status with its type and a short sample.
  -V, --version         show program's version number and exit

EXAMPLES:
Show stats for a specific user:
        python3 main.py -u <SNAP_USER> -s

List all elements (account, stories, curated highlights, spotlights, lenses) for a specific user:
        python3 main.py -u <SNAP_USER> -l a

List stories, spotlights and generate a heatmap related to this data based on upload date:
        python3 main.py -u <SNAP_USER> -l sp -m

List stories, download everything (stories, curated highlights, spotlights, lenses) and store them to directory 'data':
        python3 main.py -u <SNAP_USER> -l s -d a -o ./data
```

### When Snapchat changes its page

SnapIntel reads every field through a path into the JSON that snapchat.com embeds in profile pages, listed in `snapintel/config.json`. When Snapchat renames or moves something, those fields come back empty or wrong without any error. Run:

```bash
snapintel --self-check
```

It loads two of Snap's own accounts and marks each path `OK`, `EMPTY` (the page has the key but this account has nothing there), `SKIPPED` or `MISSING`. For a `MISSING` path it shows where the path stops and which keys are there instead, which is usually enough to update `config.json`. The exit code is 1 when a path is `MISSING`, so it can run in a scheduled job or CI. `snapintel --self-check <USERNAME>` checks one account instead, for fields that were empty on the reference accounts.

### Arabic and other right-to-left text

Most terminals, including the Windows console, draw text strictly left to right and don't join Arabic letters, so Arabic names and bios come out reversed and broken apart. When printing to a terminal, SnapIntel joins the letters and puts right-to-left text in reading order (with `arabic-reshaper` and `python-bidi`). Output redirected to a file or another program keeps the original text. Set `SNAPINTEL_RTL=0` for a terminal that already handles right-to-left text, or `SNAPINTEL_RTL=1` to force it on.



### Todo

- Implement `watch` functionality (monitoring) to check a user every X minutes for new stories and download them if not already done.
- Fix bugs?



### Support

Do you want to support me?

You can buy me a coffee here:
<a href="https://www.buymeacoffee.com/krowz" target="_blank"><img src="https://cdn.buymeacoffee.com/buttons/v2/default-yellow.png" alt="Buy Me A Coffee" height="50" ></a> 


Thanks in advance to anyone donating ❤️
