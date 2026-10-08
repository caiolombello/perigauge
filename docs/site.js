// Melhoria progressiva: botões de copiar comandos. Sem rede, sem armazenamento, sem rastreamento.
(() => {
  if (!navigator.clipboard || !window.isSecureContext) return;
  for (const button of document.querySelectorAll("[data-copy-target]")) {
    const source = document.getElementById(button.dataset.copyTarget);
    const status = document.getElementById(button.dataset.copyStatus);
    if (!source || !status) continue;
    const commands = source.textContent.split("\n").filter((l) => l.trim() && !l.trim().startsWith("#")).join("\n");
    let timer;
    button.hidden = false;
    button.addEventListener("click", async () => {
      clearTimeout(timer);
      try { await navigator.clipboard.writeText(`${commands}\n`); status.textContent = button.dataset.copied; }
      catch { status.textContent = button.dataset.failed; }
      timer = setTimeout(() => { status.textContent = ""; }, 3000);
    });
  }
})();
