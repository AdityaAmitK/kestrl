document.addEventListener('click', async (event) => {
  const button = event.target.closest('[data-copy]');
  if (!button) return;
  const source = button.closest('.code-block, .install')?.querySelector('pre, code');
  if (!source) return;
  try {
    await navigator.clipboard.writeText(source.textContent);
    button.textContent = 'Copied';
  } catch {
    button.textContent = 'Select and copy';
  }
  window.setTimeout(() => { button.textContent = 'Copy'; }, 1400);
});
