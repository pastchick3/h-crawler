# EH Crawler

``` PowerShell
# EH Crawler will search the current directory for `eh-credential`, which
# contains cookies (`ipb_member_id` and `ipb_pass_hash`) for ExHentai login.
#
# `<range>` is optional and the default range is the whole gallery. To set
# `<range>` you should follow the format like `1-5`, which means only crawling
# images from page 1 to page 5. The page index starts from 1 and the range is
# inclusive in both sides.
> eh-crawler <gallery_id>/<gallery_token>/(<range>)
```
