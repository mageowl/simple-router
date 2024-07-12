window.Router = {
  joinPath(...args) {
    args
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
  async goto(href) {
    console.log(href);
    let dataURL =
      (href.endsWith(".html")
        ? href.slice(0, -5)
        : Router.joinPath(href, "/index")) + ".page.json";
    const page = await (await fetch(dataURL)).json();
    console.log(page);

    Object.entries(page).forEach(([prop, value]) => {
      console.log(prop, value);
      document.querySelectorAll(`[data-sr-prop="${prop}"]`).forEach((el) => {
        el.innerHTML = value;
      });
    });
  },
};

window.addEventListener("load", Router.updateLinks);
