// Draggable Capture Frame Overlay
export class CaptureOverlay {
  constructor() {
    this.overlay = null;
    this.frame = null;
    this.isDragging = false;
    this.isResizing = false;
    this.startX = 0;
    this.startY = 0;
    this.frameX = 100;
    this.frameY = 100;
    this.frameW = 800;
    this.frameH = 600;
    this.resolveCallback = null;
  }
  
  show() {
    return new Promise((resolve) => {
      this.resolveCallback = resolve;
      this.createOverlay();
      this.attachEvents();
    });
  }
  
  createOverlay() {
    this.overlay = document.createElement('div');
    this.overlay.className = 'capture-overlay';
    
    this.frame = document.createElement('div');
    this.frame.className = 'capture-frame';
    this.frame.style.left = `${this.frameX}px`;
    this.frame.style.top = `${this.frameY}px`;
    this.frame.style.width = `${this.frameW}px`;
    this.frame.style.height = `${this.frameH}px`;
    
    const handle = document.createElement('div');
    handle.className = 'capture-handle';
    
    const dimensions = document.createElement('div');
    dimensions.className = 'capture-dimensions';
    dimensions.textContent = `${this.frameW} × ${this.frameH}`;
    this.dimensionsEl = dimensions;
    
    const controls = document.createElement('div');
    controls.className = 'capture-controls';
    
    const captureBtn = document.createElement('button');
    captureBtn.className = 'capture-btn';
    captureBtn.textContent = 'Start Recording';
    captureBtn.onclick = () => this.confirm();
    
    const cancelBtn = document.createElement('button');
    cancelBtn.className = 'capture-btn cancel';
    cancelBtn.textContent = 'Cancel';
    cancelBtn.onclick = () => this.cancel();
    
    controls.appendChild(captureBtn);
    controls.appendChild(cancelBtn);
    
    this.frame.appendChild(handle);
    this.frame.appendChild(dimensions);
    this.frame.appendChild(controls);
    this.overlay.appendChild(this.frame);
    document.body.appendChild(this.overlay);
    
    this.handle = handle;
  }
  
  attachEvents() {
    // Drag frame
    this.frame.addEventListener('mousedown', (e) => {
      if (e.target === this.handle) return;
      if (e.target.tagName === 'BUTTON') return;
      
      this.isDragging = true;
      this.startX = e.clientX - this.frameX;
      this.startY = e.clientY - this.frameY;
      e.preventDefault();
    });
    
    // Resize frame
    this.handle.addEventListener('mousedown', (e) => {
      this.isResizing = true;
      this.startX = e.clientX;
      this.startY = e.clientY;
      e.stopPropagation();
      e.preventDefault();
    });
    
    document.addEventListener('mousemove', (e) => {
      if (this.isDragging) {
        this.frameX = e.clientX - this.startX;
        this.frameY = e.clientY - this.startY;
        this.updateFrame();
      } else if (this.isResizing) {
        const deltaX = e.clientX - this.startX;
        const deltaY = e.clientY - this.startY;
        
        this.frameW = Math.max(200, this.frameW + deltaX);
        this.frameH = Math.max(150, this.frameH + deltaY);
        
        this.startX = e.clientX;
        this.startY = e.clientY;
        
        this.updateFrame();
      }
    });
    
    document.addEventListener('mouseup', () => {
      this.isDragging = false;
      this.isResizing = false;
    });
  }
  
  updateFrame() {
    this.frame.style.left = `${this.frameX}px`;
    this.frame.style.top = `${this.frameY}px`;
    this.frame.style.width = `${this.frameW}px`;
    this.frame.style.height = `${this.frameH}px`;
    this.dimensionsEl.textContent = `${Math.round(this.frameW)} × ${Math.round(this.frameH)}`;
  }
  
  confirm() {
    const bounds = {
      x: this.frameX,
      y: this.frameY,
      width: Math.round(this.frameW),
      height: Math.round(this.frameH)
    };
    
    if (this.resolveCallback) {
      this.resolveCallback(bounds);
    }
  }
  
  cancel() {
    if (this.resolveCallback) {
      this.resolveCallback(null);
    }
    this.destroy();
  }
  
  destroy() {
    if (this.overlay && this.overlay.parentNode) {
      this.overlay.parentNode.removeChild(this.overlay);
    }
  }
}