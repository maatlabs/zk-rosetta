// Client-side filtering and sorting for the catalog index. Operates purely on
// the rendered table via data attributes, so the page works with no backend.
(function () {
  "use strict";

  var table = document.querySelector("table.catalog");
  if (!table) return;

  var body = table.tBodies[0];
  var rows = Array.prototype.slice.call(body.rows);
  var selects = Array.prototype.slice.call(document.querySelectorAll(".filters select"));
  var search = document.querySelector('.filters input[type="search"]');
  var reset = document.querySelector(".filters .reset");
  var counter = document.getElementById("visible-count");

  function apply() {
    var active = selects
      .filter(function (s) { return s.value !== ""; })
      .map(function (s) { return { field: s.dataset.field, value: s.value }; });
    var query = (search && search.value ? search.value : "").trim().toLowerCase();

    var visible = 0;
    rows.forEach(function (row) {
      var matchesSelects = active.every(function (f) {
        return row.dataset[f.field] === f.value;
      });
      var matchesQuery = query === "" || row.textContent.toLowerCase().indexOf(query) !== -1;
      var show = matchesSelects && matchesQuery;
      row.hidden = !show;
      if (show) visible += 1;
    });
    if (counter) counter.textContent = String(visible);
  }

  selects.forEach(function (s) { s.addEventListener("change", apply); });
  if (search) search.addEventListener("input", apply);
  if (reset) {
    reset.addEventListener("click", function () {
      selects.forEach(function (s) { s.value = ""; });
      if (search) search.value = "";
      apply();
    });
  }

  var headers = Array.prototype.slice.call(table.querySelectorAll("th[data-sort]"));
  headers.forEach(function (th, index) {
    th.tabIndex = 0;
    th.setAttribute("role", "button");
    function sort() {
      var direction = th.dataset.dir === "asc" ? "desc" : "asc";
      headers.forEach(function (other) { delete other.dataset.dir; });
      th.dataset.dir = direction;
      var ordered = rows.slice().sort(function (a, b) {
        var av = a.cells[index].textContent.trim().toLowerCase();
        var bv = b.cells[index].textContent.trim().toLowerCase();
        return direction === "asc" ? av.localeCompare(bv) : bv.localeCompare(av);
      });
      ordered.forEach(function (row) { body.appendChild(row); });
    }
    th.addEventListener("click", sort);
    th.addEventListener("keydown", function (event) {
      if (event.key === "Enter" || event.key === " ") {
        event.preventDefault();
        sort();
      }
    });
  });
})();
