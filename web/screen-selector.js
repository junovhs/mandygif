// Screen/Window Source Selector
const $ = (s) => document.querySelector(s);

export class ScreenSelector {
  constructor() {
    this.modal = $('#source-selector-modal');
    this.sourceList = $('#source-list');
    this.selectedSource = null;
    
    this.modal.querySelector('.modal-close').onclick = () => this.hide();
  }
  
  async selectSource() {
    return new Promise(async (resolve) => {
      this.selectedSource = null;
      
      const sources = await window.electronAPI.getSources();
      this.renderSources(sources, resolve);
      
      this.modal.classList.remove('hidden');
    });
  }
  
  renderSources(sources, resolve) {
    this.sourceList.innerHTML = '';
    
    sources.forEach(source => {
      const item = document.createElement('div');
      item.className = 'source-item';
      
      const thumbnail = document.createElement('img');
      thumbnail.className = 'source-thumbnail';
      thumbnail.src = source.thumbnail;
      
      const name = document.createElement('div');
      name.className = 'source-name';
      name.textContent = source.name;
      
      item.appendChild(thumbnail);
      item.appendChild(name);
      
      item.onclick = () => {
        this.selectedSource = source;
        this.hide();
        resolve(source);
      };
      
      this.sourceList.appendChild(item);
    });
  }
  
  hide() {
    this.modal.classList.add('hidden');
    if (!this.selectedSource) {
      // User cancelled
      this.selectedSource = null;
    }
  }
}