window.router = {
  pageCache: {},
  /** @type {String} Current path. Always starts with '/'.*/
  path: location.pathname,

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

  /** @param {String} href Link to page relative to window.origin */
  anchor(href) {
    const a = document.createElement("a");
    a.addEventListener("click", (e) => {
      e.preventDefault();
      router.goto(href, {}, true);
    });
    return a;
  },

  /** @param {String} href Link to page relative to window.origin */
  goto(href, state = {}, includesOrigin = false) {
    router.path = "/" + router.joinPath(
      "",
      includesOrigin ? href.slice(location.origin.length) : href,
    );

    const dataURL = router.joinPath(
      includesOrigin ? "" : location.origin,
      (href.endsWith(".html")
        ? href.slice(0, -5)
        : router.joinPath(href, "/index")) + ".page.json",
    );

    return router
      ._load(dataURL)
      .then(() => {
        history.pushState({ ...state, dataURL }, "", href);
        window.dispatchEvent(new CustomEvent("navigate", { page: href }));
      })
      .catch(() => {
        location.href = router.joinPath(
          includesOrigin ? "" : location.origin,
          href,
        );
      });
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
  },
};

if (config.updateAnchors) {
  window.addEventListener("load", router._updateLinks);
  window.addEventListener("navigate", router._updateLinks);
}

window.addEventListener("popstate", (e) => {
  if (e.state.dataURL != null) {
    router._load(e.state.dataURL);
  }
});
