document.addEventListener('click', async (event) => {
  const copyButton = event.target.closest('[data-copy]');
  if (copyButton) {
    const block = copyButton.closest('.code-block')?.querySelector('pre code, pre');
    if (!block) return;
    try {
      await navigator.clipboard.writeText(block.textContent);
      const old = copyButton.textContent;
      copyButton.textContent = 'Copied';
      window.setTimeout(() => { copyButton.textContent = old; }, 1400);
    } catch {
      copyButton.textContent = 'Select and copy';
    }
  }

  const actorButton = event.target.closest('[data-actor]');
  if (actorButton) {
    document.querySelectorAll('[data-actor]').forEach((button) => {
      button.setAttribute('aria-pressed', String(button === actorButton));
    });
    const actor = actorButton.dataset.actor === 'codex' ? 'codex' : 'me';
    document.querySelector('#actor-name').textContent = actor;
    document.querySelector('#actor-name-visible').textContent = actor;
    const output = document.querySelector('#actor-output');
    if (output) {
      output.textContent = JSON.stringify({
        id: '01J8Q6M2W7R4',
        status: 200,
        ms: 84,
        body: { ok: true },
        redacted: ['local_token'],
        actor
      }, null, 2);
    }
  }
});
