# eh-manager

A CLI tool that manages my EH resources.

## Usage

``` PowerShell
# Start the application.
#
# One options is supported:
# --resource: path to the directory containing the database and galleries.
#       The default path is the current working directory.
#
# One flag is supported:
# --debug: print logs.
> eh-manager <username> <password>

# Print the database status and galleries that are not yet completely archived
# to the local disk.
> status

# Add a new gallery into the database and start to download images.
#
# `<range>` is optional and the default range is the whole gallery. To set
# `<range>` you should follow the format like `1-5`, which means only downloading
# from page 1 to page 5. The page index starts from 1 and the range is inclusive
# in both sides.
#
# If `<artist>` or `<title>` contains whitespaces, you should enclose them in
# double quotation marks.
> add <artist> <title> <url> <range>

# Remove a specific gallery from the database and delete corresponding local images.
#
# The wildcard character `*` can be used in either field.
#
# If `<artist>` or `<title>` contains whitespaces, you should enclose them in
# double quotation marks.
> remove <artist> <title>

# Find specific galleries in the database.
#
# The wildcard character `*` can be used in either field.
#
# If `<artist>` or `<title>` contains whitespaces, you should enclose them in
# double quotation marks.
> find <artist> <title>

# Exit the application.
> exit
```
