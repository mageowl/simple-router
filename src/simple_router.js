window.Router = {
  pageCache: {},


  joinPath(...args) {
    return args
      .map((part, i) => {
        if (i === 0) {
          return part.trim().replace(/[\/]*$/g, "");
        } else {
          return part.trim().replace(/(^[\/]*|[\/]*$)/g, "");
        }
      })
      .filter((x) => x.length)
      .join("/");
  },

  updateLinks() {
    for (a of document.querySelectorAll("a")) {
      if (a.host == location.host) {
        a.addEventListener("click", (e) => {
          e.preventDefault();
          Router.goto(a.href);
        });
      }
    }
  },

  /** @param {String} href */
  async goto(href, state = {}) {
    let dataURL =
      (href.endsWith(".html")
        ? href.slice(0, -5)
        : Router.joinPath(href, "/index")) + ".page.json";
    const page = Router.pageCache[dataURL] ?? await (await fetch(dataURL)).json();

    Object.entries(page).forEach(([prop, value]) => {
      document.querySelectorAll(`[data-sr-prop="${prop}"]`).forEach((el) => {
        el.innerHTML = value;
      });
    });
    Router.pageCache[dataURL] = page;
    Router.updateLinks();

    history.pushState(state, "", href);
  },
};

window.addEventListener("load", Router.updateLinks);
