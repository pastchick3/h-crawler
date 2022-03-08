# H Crawler

`H Crawler` can be tuned using a config file, whose default path is `./h-config.toml`. This config file can contain following sections and fields.

| Section | Field | Value Type | Is Required (Default) | Description |
| --- | --- | --- | --- | --- |
| `general` | `output` | String | No (`.`) | Path to store downloaded contents |
| `general` | `verbose` | Boolean | No (`false`) | Debugging output |
| `general` | `timeout` | Integer | No (`15`) | Overall timeout for a single request |
| `general` | `retry` | Integer | No (`1`) | Retrying times for a single request |
| `general` | `concurrency` | Integer | No (`5`) | Maximum number of concurrent requests |
| `exhentai` | `reload` | Integer | No (`1`) | Reloading times for a single image[^1] |
| `exhentai` | `ipb_member_id` | String | Yes | A cookie for ExHentai login |
| `exhentai` | `ipb_pass_hash` | String | Yes | A cookie for ExHentai login |

[^1]: This corresponds to the `Click here if the image fails loading` button, which will try to fetch the image from another server.

## ExHentai

``` bash
$ h-crawler exhentai <gallery_id>/<gallery_token>/[<range>]...
```

`<range>` is optional and the default range is the whole gallery. To set `<range>` you should follow the format like `1-5`, which means only crawling images from page 1 to page 5. The page index starts from 1 and the range is inclusive in both sides.

## pixiv

``` bash
$ h-crawler pixiv [--user <id>...] [--artwork <id>...]
```

If an artwork contains only a single image, it will NOT be stored in a separate folder.
