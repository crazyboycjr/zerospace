// ZeroSpace's homepage: the theme toggle, the illustration and any clip that
// play while they are seen, and the copy buttons.

(function () {
  "use strict";
  var root = document.documentElement;
  var system = window.matchMedia("(prefers-color-scheme: dark)");
  var still = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  var watch = "IntersectionObserver" in window;

  // Light or dark. A choice that matches the system's goes back to following
  // the system, so nothing is stored for it.
  function systemTheme() { return system.matches ? "dark" : "light"; }
  function shownTheme() { return root.dataset.theme || systemTheme(); }
  var toggle = document.querySelector(".theme-toggle");
  function describe() {
    if (!toggle) return;
    var theme = shownTheme();
    var following = !root.dataset.theme;
    var other = theme === "dark" ? "light" : "dark";
    toggle.setAttribute("aria-pressed", theme === "dark" ? "true" : "false");
    toggle.title = (theme === "dark" ? "Dark" : "Light") + " theme" +
      (following ? ", as the system is" : ", chosen here") + ". Click for " + other + ".";
  }
  if (toggle) {
    toggle.addEventListener("click", function () {
      var next = shownTheme() === "dark" ? "light" : "dark";
      try {
        if (next === systemTheme()) {
          delete root.dataset.theme;
          localStorage.removeItem("zerospace-theme");
        } else {
          root.dataset.theme = next;
          localStorage.setItem("zerospace-theme", next);
        }
      } catch (e) {
        root.dataset.theme = next;
      }
      describe();
    });
  }
  if (system.addEventListener) system.addEventListener("change", describe);
  describe();

  // The illustration plays while it is on screen, and its button pauses it.
  // With motion to be reduced it holds a still until the button is pressed.
  document.querySelectorAll("[data-illus]").forEach(function (figure) {
    var button = figure.parentNode.querySelector(".illus-control");
    var held = still;
    var seen = !watch;
    function update() {
      figure.classList.toggle("is-paused", held || !seen);
      if (!button) return;
      button.dataset.state = held ? "paused" : "playing";
      button.setAttribute("aria-label", held ? "Play the illustration" : "Pause the illustration");
    }
    if (button) {
      button.hidden = false;
      button.addEventListener("click", function () {
        held = !held;
        if (!held) figure.classList.add("force-motion");
        update();
      });
    }
    if (watch) {
      new IntersectionObserver(function (entries) {
        entries.forEach(function (entry) { seen = entry.isIntersecting; update(); });
      }, { threshold: 0.35 }).observe(figure);
    }
    update();
  });

  // Clips, once there are recordings: each poster loads as it comes near, and
  // a clip plays while it is on screen. With motion to be reduced it never
  // starts by itself: it shows its poster and its controls.
  var videos = Array.prototype.slice.call(document.querySelectorAll("video.clip"));
  function poster(v) { if (v.dataset.poster && !v.poster) v.poster = v.dataset.poster; }
  if (watch) {
    var near = new IntersectionObserver(function (entries) {
      entries.forEach(function (entry) {
        if (entry.isIntersecting) { poster(entry.target); near.unobserve(entry.target); }
      });
    }, { rootMargin: "800px 0px" });
    videos.forEach(function (v) { near.observe(v); });
  } else {
    videos.forEach(poster);
  }
  if (still || !watch) {
    videos.forEach(function (v) { v.controls = true; });
  } else {
    var playing = new IntersectionObserver(function (entries) {
      entries.forEach(function (entry) {
        var v = entry.target;
        if (entry.isIntersecting) {
          if (v.preload === "none") v.preload = "auto";
          var started = v.play();
          if (started && started.catch) started.catch(function () { v.controls = true; });
        } else {
          v.pause();
        }
      });
    }, { threshold: 0.35 });
    videos.forEach(function (v) { playing.observe(v); });
  }

  // Copy buttons: each copies the code of its card.
  document.querySelectorAll(".copy").forEach(function (button) {
    button.addEventListener("click", function () {
      var card = button.closest(".install-item");
      var code = card && card.querySelector("pre code");
      if (!code || !navigator.clipboard) return;
      navigator.clipboard.writeText(code.textContent).then(function () {
        button.textContent = "Copied";
        setTimeout(function () { button.textContent = "Copy"; }, 1600);
      }, function () {});
    });
  });
})();
