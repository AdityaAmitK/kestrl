document.addEventListener('click', async (event) => {
  const agentButton = event.target.closest('[data-copy-agent]');
  if (agentButton) {
    await copyAgentPrompt(agentButton);
    return;
  }
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

// The prompt lives in the page (under "See what it asks") so it can be read and checked before pasting.
let agentStatusTimer;

async function copyAgentPrompt(button) {
  const status = document.getElementById('agent-status');
  const prompt = document.getElementById(button.dataset.copyAgent);
  if (!prompt) return;
  try {
    await navigator.clipboard.writeText(prompt.textContent);
    status.textContent = 'Copied — paste it into your agent';
  } catch {
    status.textContent = 'Could not copy. Open “See what it asks” and copy the text.';
  }
  window.clearTimeout(agentStatusTimer);
  agentStatusTimer = window.setTimeout(() => { status.textContent = ''; }, 4000);
}
