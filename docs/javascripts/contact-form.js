/* A static-site contact form: prepare a draft, never claim an email was sent. */
function installContactForm() {
  document.querySelectorAll('[data-pq-contact]').forEach((form) => {
    if (form.dataset.ready) return;
    form.dataset.ready = 'true';
    const status = form.querySelector('[data-pq-contact-status]');
    const copy = form.querySelector('[data-pq-contact-copy]');
    const submit = form.querySelector('button[type="submit"]');
    const draft = () => {
      const value = (name) => form.elements.namedItem(name).value.trim();
      // Subjects stay on one line, including values pasted from other apps.
      const subject = '[PQ-CP-ABE] ' + value('subject').replace(/[\r\n]+/g, ' ');
      const body = `From: ${value('name')}\nReply to: ${value('email')}\n\n${value('message')}`;
      return { subject, body };
    };
    const valid = () => {
      // Native required validation also needs to reject whitespace-only values.
      for (const name of ['name', 'email', 'subject', 'message']) {
        const field = form.elements.namedItem(name);
        field.setCustomValidity(field.value.trim() ? '' : 'Please fill out this field.');
      }
      return form.reportValidity();
    };
    form.addEventListener('input', (event) => {
      event.target.setCustomValidity?.('');
      status.textContent = '';
    });
    form.addEventListener('submit', (event) => {
      event.preventDefault();
      if (!valid()) return;
      const { subject, body } = draft();
      const recipient = form.getAttribute('action');
      window.location.href = `${recipient}?subject=${encodeURIComponent(subject)}&body=${encodeURIComponent(body)}`;
      status.textContent = 'Continue in your email app to review and send. If it does not open, copy the message and paste it into your webmail.';
    });
    copy.addEventListener('click', async () => {
      if (!valid()) return;
      const { subject, body } = draft();
      try {
        await navigator.clipboard.writeText(`To: iam.elio2415@gmail.com\nSubject: ${subject}\n\n${body}`);
        status.textContent = 'Draft copied. Paste it into your email app and send when ready.';
      } catch {
        status.textContent = 'Clipboard access is unavailable. Copy your message manually and email iam.elio2415@gmail.com.';
      }
    });
    submit.disabled = false;
    copy.disabled = false;
  });
}

if (typeof document$ !== 'undefined') {
  document$.subscribe(installContactForm);
} else if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', installContactForm, { once: true });
} else {
  installContactForm();
}
