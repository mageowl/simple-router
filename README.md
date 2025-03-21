# simple router

A very, _very_ rudementary [SSG](https://www.cloudflare.com/learning/performance/static-site-generator) built in Rust.

## Configuration

The configuration file is located at `simple-router.toml`, and must be created for the application to work.

```toml
library_version = "0.2" # required! make sure this is up to date with the *major* version of the crate.

[out] # required!
path = "path/to/output/" # required! path to output directory
lib_file = "simple-router.js" # optional. name of JS library file relative to output directory

[source] # optional.
static_path = "." # path to the static directory (files that will not be modified by simple router)
pages_path = "./pages/" # path to pages directory.
template = "layout.html" # path to template HTML file
exclude = [] # list of paths to exclude from 

[xml] # optional.
ignore_comments = true # remove comments from html

[js] # optional.
update_anchors = true # automatically update all <a> elements to use the router.
not_found = "404.html" # path to 404 page. needs to be the same as hosting provider's!

[scripts] # optional.
prebuild = "echo hello world" # command is run using sh, before simple-router does anything.
postbuild = "echo goodbye world" # after all files are in the docs folder.
```

## Templating

By default, `layout.html` is a special file that contains the template for the page. All files inside the ./pages/ folder by default are considered pages.

```html
<!-- layout.html -->

<html>
    <head>
        <!-- This is a placeholder (denoted by sr-prop="name").
             When the page gets loaded, the contents of this element will be replaced. -->
        <title sr-prop="title" /> 
    </head>
    <body>
        <!-- Properties starting with '__' are special. 
             - `__page` = Current path -->
        <h1 sr-prop="__page" /> 
        <div sr-prop="content" />
    </body>
</html>

<!-- pages/index.html -->

<content> <!-- Elements at the root of a page are considered "properties" -->
    <!-- Placeholders will be filled in using properties of the same name. -->
    <p>Templating is so cool!</p> 
</content>
<title>Hello World</title>  <!-- Placeholders can contain both plain text and html. -->

```

## JavaScript Interface

The JavaScript library creates a `window.router` property that lets you navigate to pages. By default, all anchor elements (`a`) that link to local pages will automatically be updated to use the interface.

```javascript
window.router.goto("/cat.html"); // Navigate to cat.html

window.router.goto("/"); // Navigate to the root (index.html)

window.router.goto("/cat"); // Navigate to the cat folder (cat/index.html)

// Create an anchor element and set its href to '/cat.html'.
// Note: the href attribute doesn't actually affect where this link will go.
const anchorElement = window.router.anchor("/cat.html");

await window.router.goto("/about.html"); // Wait for the about page to load, then continue.

console.log(router.path); // Print current path.

history.back(); // Go back.
```

Additionally, there is JSDoc in src/simple_router.js.
