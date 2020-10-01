# eh-manager

A command line tool that manages my EH resources.

## Usage

``` PowerShell
# Start the application.
> eh-manager <ipb_member_id> <ipb_pass_hash>

# Add a new gallery into the database and start to download images.
#
# `<range>` is optional and the default range is the whole gallery. To set
# `<range>` you should follow the format like `1-5`, which means only downloading
# images from page 1 to page 5. The page index starts from 1 and the range is
# inclusive in both sides.
#
# If `<artist>` or `<title>` contains whitespaces, you should enclose them in
# double quotation marks (e.g. `add "an artist" title url`).
> add <artist> <title> <url> <range>

# Find specific galleries in the database.
#
# The wildcard character `*` can be used in either field (e.g. `find artist *`).
#
# If `<artist>` or `<title>` contains whitespaces, you should enclose them in
# double quotation marks (e.g. `find "an artist" title`).
> find <artist> <title>

# Remove a specific gallery from the database and delete corresponding local images.
#
# The wildcard character `*` can be used in either field (e.g. `remove artist *`).
#
# If `<artist>` or `<title>` contains whitespaces, you should enclose them in
# double quotation marks (e.g. `remove "an artist" title`).
> remove <artist> <title>

# Exit the application.
> exit
```
