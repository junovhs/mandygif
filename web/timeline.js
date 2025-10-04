// Timeline Trimming Control
export class Timeline {
  constructor(container, frames) {
    this.container = container;
    this.frames = frames;
    this.startFrame = 0;
    this.endFrame = frames.length;
    this.onTrim = null;
    
    this.render();
  }
  
  render() {
    this.container.innerHTML = '';
    
    const track = document.createElement('div');
    track.style.cssText = `
      position: relative;
      height: 100%;
      background: #1a1a1a;
      border-radius: 4px;
      overflow: hidden;
    `;
    
    // Frame thumbnails
    const thumbContainer = document.createElement('div');
    thumbContainer.style.cssText = `
      display: flex;
      height: 100%;
      opacity: 0.5;
    `;
    
    const thumbInterval = Math.max(1, Math.floor(this.frames.length / 20));
    for (let i = 0; i < this.frames.length; i += thumbInterval) {
      const thumb = document.createElement('div');
      thumb.style.cssText = `
        flex: 1;
        background: #333;
        border-right: 1px solid #1a1a1a;
      `;
      thumbContainer.appendChild(thumb);
    }
    
    track.appendChild(thumbContainer);
    
    // Trim handles
    const trimOverlay = document.createElement('div');
    trimOverlay.style.cssText = `
      position: absolute;
      top: 0;
      bottom: 0;
      background: rgba(59, 130, 246, 0.2);
      border-left: 3px solid #3b82f6;
      border-right: 3px solid #3b82f6;
      pointer-events: none;
    `;
    
    this.trimOverlay = trimOverlay;
    track.appendChild(trimOverlay);
    
    // Start handle
    const startHandle = document.createElement('div');
    startHandle.style.cssText = `
      position: absolute;
      left: 0;
      top: 0;
      bottom: 0;
      width: 20px;
      background: #3b82f6;
      cursor: ew-resize;
      display: flex;
      align-items: center;
      justify-content: center;
      color: white;
      font-size: 10px;
    `;
    startHandle.innerHTML = '◀';
    
    // End handle
    const endHandle = document.createElement('div');
    endHandle.style.cssText = `
      position: absolute;
      right: 0;
      top: 0;
      bottom: 0;
      width: 20px;
      background: #3b82f6;
      cursor: ew-resize;
      display: flex;
      align-items: center;
      justify-content: center;
      color: white;
      font-size: 10px;
    `;
    endHandle.innerHTML = '▶';
    
    track.appendChild(startHandle);
    track.appendChild(endHandle);
    
    this.container.appendChild(track);
    
    this.updateTrimOverlay();
    this.attachDragHandlers(startHandle, endHandle, track);
  }
  
  attachDragHandlers(startHandle, endHandle, track) {
    let dragging = null;
    
    const onMouseDown = (handle, type) => (e) => {
      dragging = type;
      e.preventDefault();
    };
    
    startHandle.addEventListener('mousedown', onMouseDown(startHandle, 'start'));
    endHandle.addEventListener('mousedown', onMouseDown(endHandle, 'end'));
    
    document.addEventListener('mousemove', (e) => {
      if (!dragging) return;
      
      const rect = track.getBoundingClientRect();
      const x = e.clientX - rect.left;
      const percent = Math.max(0, Math.min(1, x / rect.width));
      const frame = Math.floor(percent * this.frames.length);
      
      if (dragging === 'start') {
        this.startFrame = Math.min(frame, this.endFrame - 1);
      } else {
        this.endFrame = Math.max(frame, this.startFrame + 1);
      }
      
      this.updateTrimOverlay();
      
      if (this.onTrim) {
        this.onTrim(this.startFrame, this.endFrame);
      }
    });
    
    document.addEventListener('mouseup', () => {
      dragging = null;
    });
  }
  
  updateTrimOverlay() {
    const startPercent = (this.startFrame / this.frames.length) * 100;
    const endPercent = (this.endFrame / this.frames.length) * 100;
    
    this.trimOverlay.style.left = `${startPercent}%`;
    this.trimOverlay.style.right = `${100 - endPercent}%`;
  }
  
  getRange() {
    return { start: this.startFrame, end: this.endFrame };
  }
  
  getFrameCount() {
    return this.endFrame - this.startFrame;
  }
}