# H Crawler

`H Crawler` can be tuned using a config file, whose default path is `--config=./h-config.toml`. This config file can contain following sections and fields. All of them can also be supplied directly from the command line, which will override values in the config file. Also, `H Crawler` uses `env_logger` for logging, so you can set `RUST_LOG=h_crawler` appropriately for more detailed output.

| Section | Field | Value Type | Is Required (Default) | Description |
| --- | --- | --- | --- | --- |
| N/A | `output` | String | No (`.`) | Path to store downloaded contents |
| N/A | `timeout` | Integer | No (`15`) | Overall timeout for requests |
| N/A | `retry` | Integer | No (`1`) | Retrying times for requests |
| N/A | `concurrency` | Integer | No (`5`) | Maximum concurrent requests |
| `exhentai` | `reload` | Integer | No (`1`) | Reloading times for images[^1] |
| `exhentai` | `ipb_member_id` | String | Yes | Cookie for ExHentai login |
| `exhentai` | `ipb_pass_hash` | String | Yes | Cookie for ExHentai login |
| `pixiv` | `PHPSESSID` | String | Yes | Cookie for pixiv login |

[^1]: This corresponds to the `Click here if the image fails loading` button, which will try to fetch the image from another server.

## ExHentai

``` bash
$ h-crawler exhentai <gallery_id>/<gallery_token>/[<range>]...
```

`<range>` is optional and the default range is the whole gallery. To set `<range>` you should follow the format like `1-5`, which means only crawling images from page 1 to page 5. The page index starts from 1 and the range is inclusive in both sides.

## pixiv

``` bash
$ h-crawler pixiv user <id>...
$ h-crawler pixiv artwork <id>...
```

If an artwork contains only one image, it will NOT be stored in a separate folder.
