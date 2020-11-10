# EH Crawler

``` PowerShell
# EH Crawler will search the current directory for `eh-credential`.
> eh-crawler

# `<range>` is optional and the default range is the whole gallery. To set
# `<range>` you should follow the format like `1-5`, which means only crawling
# images from page 1 to page 5. The page index starts from 1 and the range is
# inclusive in both sides.
> crawl <url> <range>

# Show galleries that are being crawled with their associated IDs.
> status

# Show status of a specific gallery.
> status <id>

# Cancel a gallery.
> cancel <id>

# Exit the application.
> exit
```
