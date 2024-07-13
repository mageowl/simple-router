# simple router

A very, _very_ rudementary [SSG](https://www.cloudflare.com/learning/performance/static-site-generator) built in Rust.

## Configuration

The configuration file is located at `simple-router.toml`, and must be created for the application to work.

```toml
[out]
path = "path/to/output/" # required! path to output directory
lib_file = "simple-router.js" # optional. name of JS libraries relative to output directory

[source] # optional.
path = "." # path to the source directory
template = "layout.html" # path to template HTML file
exclude = [] # list of paths to exclude from 

[xml] # optional.
ignore_comments = true # remove comments from html
```
