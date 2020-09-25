# eh-manager

A CLI tool that manages my EH resources.

## Usage

``` PowerShell
# Start the application.
#
# Two options are supported:
# --resource: path to the directory containing the database and galleries.
#       The default path is the current working directory.
# --log: path to the directory to place error logs.
#       The default path is the current working directory.
#
# One flag is supported:
# --debug: print all logs.
> eh-manager <username> <password>

# Print the database status and galleries that are being downloaded.
> status

# Add a new gallery into the database, and start to download images to the local disk.
> add <url>

# Query the database for specific galleries.
#
# Three fields are supported: `title`, `group`, and `artist`.
# `title` can be either English or Japanese and it is not required to be complete.
# `group` and `artist` must be exactly the same as corresponding EH tags.
#
# This command returns a list of galleries with their id, basic information, and
# paths on the local disk.
> query <field> <keyward>

# Remove a specific gallery from the database and delete corresponding local images.
#
# You must comfirm this command again to make it complete.
> remove <id>

# Exit the application.
> exit
```
