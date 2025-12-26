window.addEventListener('DOMContentLoaded', () => {
    const { invoke } = window.__TAURI__.core;
    const loadFilesBtn = document.getElementById('loadFilesBtn');
    const content = document.getElementById('content');

    async function loadFiles() {
        try {
            loadFilesBtn.disabled = true;
            loadFilesBtn.textContent = 'Loading...';
            
            const html = await invoke('list_files_html');
            content.innerHTML = html;
            
        } catch (error) {
            content.innerHTML = `<p class="error">Error: ${error}</p>`;
        } finally {
            loadFilesBtn.disabled = false;
            loadFilesBtn.textContent = 'List Files';
        }
    }

    loadFilesBtn.addEventListener('click', loadFiles);
});
