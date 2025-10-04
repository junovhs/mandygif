const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('electronAPI', {
  getSources: () => ipcRenderer.invoke('get-sources'),
  
  showCaptureOverlay: () => ipcRenderer.invoke('show-capture-overlay'),
  
  confirmCapture: (bounds) => ipcRenderer.invoke('capture-confirmed', bounds),
  
  cancelCapture: () => ipcRenderer.invoke('capture-cancelled'),
  
  startRecording: (config) => ipcRenderer.invoke('start-recording', config),
  
  stopRecording: (frames) => ipcRenderer.invoke('stop-recording', frames),
  
  exportWebP: (data) => ipcRenderer.invoke('export-webp', data),
  
  exportMP4: (data) => ipcRenderer.invoke('export-mp4', data),
  
  exportGIF: (data) => ipcRenderer.invoke('export-gif', data),
  
  onRecordingSourceReady: (callback) => {
    ipcRenderer.on('recording-source-ready', (event, data) => callback(data));
  },
  
  onCaptureBoundsReady: (callback) => {
    ipcRenderer.on('capture-bounds-ready', (event, data) => callback(data));
  },
  
  onCaptureCancelled: (callback) => {
    ipcRenderer.on('capture-cancelled', () => callback());
  }
});