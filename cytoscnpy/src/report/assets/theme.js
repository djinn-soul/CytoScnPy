// Report theme handling. Loaded synchronously in <head> so the chosen theme
// is applied before first paint. "auto" leaves <html> without data-theme and
// lets the prefers-color-scheme media query decide.
(function () {
  var DEFAULT_THEME = "__CYTOSCNPY_DEFAULT_THEME__";
  var STORAGE_KEY = "cytoscnpy-report-theme";
  var root = document.documentElement;
  // A page may open in its own theme, e.g. `cytoscnpy deslop --html-theme`.
  var pageDefault = root.getAttribute("data-default-theme");
  if (pageDefault) DEFAULT_THEME = pageDefault;

  function readStored() {
    try {
      return window.localStorage.getItem(STORAGE_KEY);
    } catch (e) {
      return null;
    }
  }

  function writeStored(theme) {
    try {
      window.localStorage.setItem(STORAGE_KEY, theme);
    } catch (e) {
      /* storage unavailable (private mode, file:// restrictions) */
    }
  }

  function apply(theme) {
    if (theme === "light" || theme === "dark") {
      root.setAttribute("data-theme", theme);
    } else {
      root.removeAttribute("data-theme");
    }
  }

  function systemPrefersLight() {
    return !!(window.matchMedia && window.matchMedia("(prefers-color-scheme: light)").matches);
  }

  function effective() {
    var forced = root.getAttribute("data-theme");
    if (forced === "light" || forced === "dark") return forced;
    return systemPrefersLight() ? "light" : "dark";
  }

  root.classList.remove("no-js");
  apply(readStored() || DEFAULT_THEME);

  document.addEventListener("DOMContentLoaded", function () {
    var button = document.getElementById("theme-toggle");
    if (!button) return;

    function sync() {
      var current = effective();
      var next = current === "dark" ? "light" : "dark";
      button.setAttribute("data-current", current);
      button.setAttribute("aria-label", "Switch to " + next + " theme");
      button.setAttribute("title", "Switch to " + next + " theme");
    }

    button.addEventListener("click", function () {
      var next = effective() === "dark" ? "light" : "dark";
      apply(next);
      writeStored(next);
      sync();
    });

    if (window.matchMedia) {
      var query = window.matchMedia("(prefers-color-scheme: light)");
      if (query.addEventListener) query.addEventListener("change", sync);
      else if (query.addListener) query.addListener(sync);
    }

    sync();
  });
})();
