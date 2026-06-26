/* RavenClaws.io — small progressive-enhancement script. No dependencies. */
(function () {
  "use strict";

  /* ----- header shadow on scroll ----- */
  var header = document.querySelector(".site-header");
  function onScroll() {
    if (!header) return;
    header.classList.toggle("scrolled", window.scrollY > 8);
  }
  window.addEventListener("scroll", onScroll, { passive: true });
  onScroll();

  /* ----- mobile nav toggle ----- */
  var nav = document.querySelector(".nav");
  var toggle = document.querySelector(".nav-toggle");
  if (toggle && nav) {
    toggle.addEventListener("click", function () {
      var open = nav.classList.toggle("open");
      toggle.setAttribute("aria-expanded", open ? "true" : "false");
    });
    nav.querySelectorAll(".nav-links a").forEach(function (a) {
      a.addEventListener("click", function () { nav.classList.remove("open"); toggle.setAttribute("aria-expanded", "false"); });
    });
  }

  /* ----- copy buttons on code blocks ----- */
  document.querySelectorAll(".code").forEach(function (block) {
    var btn = block.querySelector(".code__copy");
    var pre = block.querySelector("pre");
    if (!btn || !pre) return;
    btn.addEventListener("click", function () {
      var text = pre.innerText.replace(/ /g, " ");
      navigator.clipboard.writeText(text).then(function () {
        var prev = btn.textContent;
        btn.textContent = "Copied";
        btn.classList.add("copied");
        setTimeout(function () { btn.textContent = prev; btn.classList.remove("copied"); }, 1600);
      }).catch(function () {
        btn.textContent = "Press Ctrl+C";
      });
    });
  });

  /* ----- year in footer ----- */
  var y = document.querySelector("[data-year]");
  if (y) y.textContent = new Date().getFullYear();

  /* ----- docs sidebar scroll-spy ----- */
  var spy = document.querySelectorAll(".docs-side a[data-spy]");
  if (spy.length) {
    var targets = [];
    spy.forEach(function (a) {
      var id = a.getAttribute("href");
      if (id && id.charAt(0) === "#") {
        var el = document.getElementById(id.slice(1));
        if (el) targets.push({ a: a, el: el });
      }
    });
    if (targets.length && "IntersectionObserver" in window) {
      var io = new IntersectionObserver(function (entries) {
        entries.forEach(function (e) {
          if (e.isIntersecting) {
            var id = "#" + e.target.id;
            spy.forEach(function (a) { a.classList.toggle("active", a.getAttribute("href") === id); });
          }
        });
      }, { rootMargin: "-72px 0px -70% 0px", threshold: 0 });
      targets.forEach(function (t) { io.observe(t.el); });
    }
  }
})();
