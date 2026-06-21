const { listen } = window.__TAURI__.event;

// Listen for LCU Champ Select session events inside the overlay
listen('champ-select-update', (event) => {
  const data = event.payload;
  console.log("Overlay LCU Champ Select Update:", data);
});
