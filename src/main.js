const { listen } = window.__TAURI__.event;

// Listen for LCU Connection Status
listen('lcu-status', (event) => {
  const status = event.payload;
  const indicator = document.querySelector('.status-indicator');
  const statusText = document.getElementById('status-text');
  const valPort = document.getElementById('val-port');
  const valPid = document.getElementById('val-pid');
  const valAuth = document.getElementById('val-auth');
  const profileWidget = document.getElementById('profile-widget');

  if (status.connected) {
    indicator.className = 'status-indicator connected';
    statusText.textContent = 'Cliente de LoL detectado';
    
    valPort.textContent = status.port;
    valPid.textContent = status.pid;
    valAuth.textContent = 'Conectado (Local)';
    valAuth.style.color = 'var(--win-color)';
  } else {
    indicator.className = 'status-indicator disconnected';
    statusText.textContent = 'Buscando cliente de LoL...';
    
    valPort.textContent = '-';
    valPid.textContent = '-';
    valAuth.textContent = 'Desconectado';
    valAuth.style.color = 'var(--text-muted)';
    
    // Hide profile widget when disconnected
    profileWidget.style.display = 'none';
  }
});

// Listen for summoner details from League Client
listen('summoner-info', (event) => {
  const summoner = event.payload;
  const profileWidget = document.getElementById('profile-widget');
  const profileIcon = document.getElementById('profile-icon');
  const profileName = document.getElementById('profile-name');
  const profileLevel = document.getElementById('profile-level');

  if (summoner) {
    profileIcon.src = `https://ddragon.leagueoflegends.com/cdn/14.3.1/img/profileicon/${summoner.profileIconId}.png`;
    profileName.textContent = summoner.name;
    profileLevel.textContent = `Nivel ${summoner.level}`;
    profileWidget.style.display = 'flex';
  }
});

let lastMyTeamState = null;

// Listen for champion select updates
listen('champ-select-update', (event) => {
  const data = event.payload;
  const draftDescription = document.getElementById('draft-description');
  const draftTeamList = document.getElementById('draft-team-list');
  const draftStatus = document.getElementById('draft-status');

  if (data.active) {
    draftDescription.style.display = 'none';
    draftTeamList.style.display = 'flex';
    draftStatus.textContent = '¡Selección de campeones ACTIVA!';
    draftStatus.style.color = 'var(--text-main)';

    // compare serialized state to prevent DOM rewrite and image flashing when data is identical
    const stateString = JSON.stringify(data.myTeam);
    if (stateString === lastMyTeamState) {
      return;
    }
    lastMyTeamState = stateString;

    draftTeamList.innerHTML = data.myTeam.map((player, index) => {
      const imgTag = player.championImage 
        ? `<img class="draft-avatar" src="${player.championImage}" alt="">`
        : `<div class="draft-avatar" style="display: inline-block;"></div>`;
      
      return `
        <div class="draft-player-row">
          ${imgTag}
          <div class="draft-player-info">
            <span class="draft-player-title">Jugador ${index + 1}</span>
            <span class="draft-champ-name">${player.championName}</span>
          </div>
        </div>
      `;
    }).join('');
  } else {
    lastMyTeamState = null;
    draftDescription.style.display = 'block';
    draftTeamList.style.display = 'none';
    draftTeamList.innerHTML = '';
    draftStatus.textContent = 'Esperando Selección de Campeón...';
    draftStatus.style.color = 'var(--text-muted)';
  }
});

// Tab Switching logic
const navItems = {
  'nav-dashboard': 'view-dashboard',
  'nav-draft': 'view-draft',
  'nav-matches': 'view-matches',
  'nav-settings': 'view-settings'
};

Object.keys(navItems).forEach(navId => {
  document.getElementById(navId).addEventListener('click', (e) => {
    e.preventDefault();
    
    // Deactivate all nav items
    Object.keys(navItems).forEach(id => {
      document.getElementById(id).classList.remove('active');
    });
    
    // Activate clicked nav item
    document.getElementById(navId).classList.add('active');
    
    // Hide all views
    Object.values(navItems).forEach(viewId => {
      document.getElementById(viewId).style.display = 'none';
    });
    
    // Show clicked view
    document.getElementById(navItems[navId]).style.display = 'block';
  });
});

// Listen for Match History update
listen('match-history', (event) => {
  const games = event.payload;
  const matchListContainer = document.getElementById('match-list');
  if (!games || games.length === 0) {
    matchListContainer.innerHTML = `
      <div class="card">
        <p style="color: var(--text-muted); text-align: center;">No se encontraron partidas recientes en este historial.</p>
      </div>
    `;
    return;
  }

  matchListContainer.innerHTML = games.map(game => {
    const statusClass = game.win ? 'match-win' : 'match-lose';
    const statusText = game.win ? 'Victoria' : 'Derrota';
    const kdaText = `${game.kills} / ${game.deaths} / ${game.assists}`;

    return `
      <div class="card match-row ${statusClass}">
        <div class="match-left">
          <img class="match-champion-icon" src="${game.championImage || ''}" alt="">
          <div class="match-info">
            <span class="match-result">${statusText}</span>
            <span class="match-champ-name">${game.championName}</span>
          </div>
        </div>
        <div class="match-kda">
          <span class="kda-label">KDA</span>
          <span class="kda-val">${kdaText}</span>
        </div>
        <div class="match-right">
          <span class="game-id">ID: ${game.gameId}</span>
        </div>
      </div>
    `;
  }).join('');
});
