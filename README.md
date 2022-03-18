# H Crawler

## Config

`H Crawler` can be tuned with a config file, whose default path is `./h-config.toml`. This config file can contain following sections and fields. All of them can also be supplied directly from the command line, which will override values in the config file.

| Section | Field | Value Type | Is Required (Default) | Description |
| --- | --- | --- | --- | --- |
| N/A | `concurrency` | Integer | No (`8`) | Maximum concurrent requests |
| N/A | `timeout` | Integer | No (`30`) | Overall timeout for requests in seconds |
| N/A | `retry` | Integer | No (`1`) | Retrying times for requests |
| N/A | `output` | String | No (`.`) | Path to store downloaded contents |
| `exhentai` | `reload` | Integer | No (`1`) | Reloading times for images[^1] |
| `exhentai` | `ipb_member_id` | String | Yes | Cookie for ExHentai login |
| `exhentai` | `ipb_pass_hash` | String | Yes | Cookie for ExHentai login |
| `pixiv` | `phpsessid` | String | Yes | Cookie for pixiv login |

[^1]: This corresponds to the `Click here if the image fails loading` button, which will try to fetch the image from another server.

## Usage

``` bash
# EHentai
$ h-crawler exhentai <gallery_id>/<gallery_token>/[<range>]...
# pixiv
$ h-crawler pixiv user <user_id>[/<range>]...
$ h-crawler pixiv illust <illust_id>...
```

`<range>` looks like `1-5`. The range index starts from 1 and it is inclusive on both sides.

If an illust contains only one image, it will NOT be stored in a separate directory.
