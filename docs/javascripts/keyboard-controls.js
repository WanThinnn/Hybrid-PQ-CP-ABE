/* Keep the theme's checkbox toggles, expose their header actions as buttons. */
function installKeyboardControls() {
  for (const [id, name] of [['__drawer', 'Navigation'], ['__search', 'Search']]) {
    const label = document.querySelector(`.md-header label[for="${id}"]`);
    const toggle = document.getElementById(id);
    if (!label || !toggle) continue;
    const button = document.createElement('button');
    button.type = 'button';
    button.className = label.className;
    // Retain the attribute used by the theme's responsive CSS selectors.
    button.setAttribute('for', id);
    button.setAttribute('aria-label', name);
    button.innerHTML = label.innerHTML;
    button.classList.add('pq-header-toggle');
    button.addEventListener('click', () => toggle.click());
    const sync = () => button.setAttribute('aria-expanded', String(toggle.checked));
    toggle.addEventListener('change', sync);
    sync();
    label.replaceWith(button);
    document.addEventListener('keydown', (event) => {
      if (event.key === 'Escape' && toggle.checked) {
        toggle.click();
        if (button.getClientRects().length) {
          button.focus();
        }
      }
    });
  }
  // Make overflowing code/table regions reachable without a mouse.
  document.querySelectorAll('.md-typeset pre > code, .md-typeset__scrollwrap').forEach((element) => {
    if (element.scrollWidth > element.clientWidth && element.tabIndex < 0) {
      element.tabIndex = 0;
      element.setAttribute('aria-label', element.tagName === 'CODE' ? 'Scrollable code example' : 'Scrollable table');
    }
  });
}

if (typeof document$ !== 'undefined') {
  document$.subscribe(installKeyboardControls);
} else if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', installKeyboardControls, { once: true });
} else {
  installKeyboardControls();
}
