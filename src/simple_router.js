window.router = {
  pageCache: {},

  /** @param {...String} args Joins `args` together as a path. */
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

  /** Internal: Do not use */
  _updateLinks() {
    for (a of document.querySelectorAll("a")) {
      if (a.host == location.host) {
        a.addEventListener("click", (e) => {
          e.preventDefault();
          router.goto(a.href, {}, true);
        });
      }
    }
  },

  /** @param {String} href Link to page relative to root of page */
  async goto(href, state = {}, origin = false) {
    const dataURL = router.joinPath(
      origin ? "" : location.origin,
      (href.endsWith(".html")
        ? href.slice(0, -5)
        : router.joinPath(href, "/index")) + ".page.json",
    );
    await router._load(dataURL);


    history.pushState({ ...state, dataURL }, "", href);
    window.dispatchEvent(new CustomEvent("navigate", { page: href }));
  },

  /** Internal: Do not use */
  async _load(dataURL) {
    const page =
      router.pageCache[dataURL] ?? (await (await fetch(dataURL)).json());

    Object.entries(page).forEach(([prop, value]) => {
      document.querySelectorAll(`[data-sr-prop="${prop}"]`).forEach((el) => {
        el.innerHTML = value;
      });
    });
    router.pageCache[dataURL] = page;
  }
};

window.addEventListener("load", router._updateLinks);
window.addEventListener("navigate", router._updateLinks);
window.addEventListener("popstate", (e) => {
  if (e.state.dataURL != null) {
    router._load(e.state.dataURL);
  }
})
