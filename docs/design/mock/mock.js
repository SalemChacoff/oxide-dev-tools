// Theme handling for the oxide-design mock.
// System = follow prefers-color-scheme (no data-theme attribute);
// Light/Dark = forced via data-theme. Persisted in localStorage.
(function () {
  var KEY = "oxide-mock-theme";
  var selects = document.querySelectorAll(".theme-select");

  function apply() {
    var value = localStorage.getItem(KEY) || "system";
    selects.forEach(function (s) { s.value = value; });
    if (value === "light") document.documentElement.setAttribute("data-theme", "light");
    else if (value === "dark") document.documentElement.setAttribute("data-theme", "dark");
    else document.documentElement.removeAttribute("data-theme");
  }

  selects.forEach(function (s) {
    s.addEventListener("change", function (e) {
      localStorage.setItem(KEY, e.target.value);
      apply();
    });
  });

  apply();
})();
