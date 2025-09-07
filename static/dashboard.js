let currentUser = null;
let currentToken = null;
let currentModalType = null;
let guildRefreshInterval = null;
let allUserGuilds = []; // Store all user guilds (both connected and not connected)

// Discord OAuth2 Configuration
const DISCORD_CLIENT_ID = "191805365212413953"; // Replace with your actual client ID
const DISCORD_REDIRECT_URI = encodeURIComponent(
  window.location.origin + "/auth/callback",
);
const DISCORD_SCOPES = "identify guilds";

// Guild refresh interval (30 seconds)
const GUILD_REFRESH_INTERVAL = 10000;

document.addEventListener("DOMContentLoaded", function () {
  document.getElementById("discord-login").addEventListener("click", (e) => {
    e.preventDefault();
    loginWithDiscord();
  });

  document.getElementById("logout").addEventListener("click", (e) => {
    e.preventDefault();
    logout();
  });

  // Initialize
  const savedToken = localStorage.getItem("discord_token");
  if (savedToken) {
    currentToken = savedToken;
    // Validate token and get user info
    validateToken();
  }

  // Close modal when clicking outside
  document.getElementById("api-modal").addEventListener("click", (e) => {
    if (e.target.id === "api-modal") {
      closeModal();
    }
  });

  // Check if we're returning from OAuth callback
  const urlParams = new URLSearchParams(window.location.search);
  const token = urlParams.get("token");
  if (token) {
    localStorage.setItem("discord_token", token);
    currentToken = token;
    // Clean up URL
    window.history.replaceState({}, document.title, window.location.pathname);
    validateToken();
  }

  // Handle page visibility changes to pause/resume guild refresh
  document.addEventListener("visibilitychange", () => {
    if (document.hidden) {
      // Page is hidden, don't stop refresh but reduce frequency would be nice
      // For now, keep the same frequency since 30s is already reasonable
    } else {
      // Page is visible, ensure refresh is running if user is logged in
      if (currentToken && currentUser && !guildRefreshInterval) {
        startGuildRefresh();
      }
    }
  });
});

function loginWithDiscord() {
  // Redirect to Discord OAuth2
  const discordOAuthUrl = `https://discord.com/api/oauth2/authorize?client_id=${DISCORD_CLIENT_ID}&redirect_uri=${DISCORD_REDIRECT_URI}&response_type=code&scope=${DISCORD_SCOPES}`;
  window.location.href = discordOAuthUrl;
}

function logout() {
  localStorage.removeItem("discord_token");
  currentToken = null;
  currentUser = null;
  stopGuildRefresh();
  updateAuthUI();
}

function updateAuthUI() {
  const loginSection = document.getElementById("login-section");
  const userSection = document.getElementById("user-section");
  const userInfo = document.getElementById("user-info");

  if (currentUser) {
    loginSection.classList.add("hidden");
    userSection.classList.remove("hidden");
    userInfo.classList.add("visible");
    document.getElementById("user-name").textContent =
      currentUser.global_name || currentUser.username;
    enableButtons();
  } else {
    loginSection.classList.remove("hidden");
    userSection.classList.add("hidden");
    userInfo.classList.remove("visible");
    disableButtons();
  }
}

function enableButtons() {
  const buttons = [
    "guilds-btn",
    "get-queue-btn",
    "add-queue-btn",
    "skip-btn",
    "clear-queue-btn",
    "play-btn",
    "stop-btn",
    "join-btn",
    "volume-btn",
    "song-info-btn",
  ];
  buttons.forEach((id) => {
    const btn = document.getElementById(id);
    if (btn) btn.disabled = false;
  });
}

function disableButtons() {
  const buttons = [
    "guilds-btn",
    "get-queue-btn",
    "add-queue-btn",
    "skip-btn",
    "clear-queue-btn",
    "play-btn",
    "stop-btn",
    "join-btn",
    "volume-btn",
    "song-info-btn",
  ];
  buttons.forEach((id) => {
    const btn = document.getElementById(id);
    if (btn) btn.disabled = true;
  });
}

async function apiCall(method, endpoint, body = null) {
  const headers = {
    "Content-Type": "application/json",
  };

  if (currentToken) {
    headers["Authorization"] = `Bearer ${currentToken}`;
  }

  const config = {
    method,
    headers,
  };

  if (body) {
    config.body = JSON.stringify(body);
  }

  const response = await fetch(endpoint, config);
  const data = await response.json();

  return { status: response.status, data };
}

async function executeAuth() {
  if (!currentToken) {
    Swal.fire({
      icon: "warning",
      title: "Authentication Required",
      text: "Please login first",
      confirmButtonColor: "#5865f2",
    });
    return;
  }

  try {
    const result = await apiCall("POST", "/api/auth/validate", {
      access_token: currentToken,
    });
    showResponse(result.status, result.data);
  } catch (error) {
    showResponse(500, { error: error.message });
  }
}

async function executeGuilds() {
  try {
    const result = await apiCall("GET", "/api/guilds");
    showResponse(result.status, result.data);

    if (result.status === 200 && result.data.success) {
      displayUserGuilds(result.data.data);
    }
  } catch (error) {
    showResponse(500, { error: error.message });
  }
}

async function validateToken() {
  if (!currentToken) return;

  try {
    const result = await apiCall("POST", "/api/auth/validate", {
      access_token: currentToken,
    });
    if (result.status === 200 && result.data.success) {
      const userData = result.data.data;
      currentUser = userData.user;
      updateAuthUI();
      displayUserInfo(userData.user);
      displayUserGuilds(userData.guilds);
      startGuildRefresh();
    } else {
      // Token is invalid, clear it
      localStorage.removeItem("discord_token");
      currentToken = null;
      currentUser = null;
      updateAuthUI();
      stopGuildRefresh();
    }
  } catch (error) {
    console.error("Token validation failed:", error);
    // Token validation failed, clear it
    localStorage.removeItem("discord_token");
    currentToken = null;
    currentUser = null;
    updateAuthUI();
    stopGuildRefresh();
  }
}

function displayUserInfo(user) {
  const userDetails = document.getElementById("user-details");
  userDetails.innerHTML = `
        <div style="display: flex; align-items: center; gap: 12px;">
            <div class="guild-icon">${user.username[0].toUpperCase()}</div>
            <div>
                <strong>${user.global_name || user.username}</strong>
                <div style="color: var(--text-muted); font-size: 13px;">ID: ${user.id}</div>
            </div>
        </div>
    `;
}

function displayUserGuilds(guilds) {
  // Store all guilds globally for use in modals
  allUserGuilds = guilds;

  const guildList = document.getElementById("guild-list");

  // Filter to only show connected guilds
  const connectedGuilds = guilds.filter((guild) => guild.connected);

  if (connectedGuilds.length === 0) {
    guildList.innerHTML = `
            <div style="text-align: center; padding: 20px; color: var(--text-muted);">
                <div style="font-size: 18px; margin-bottom: 8px;">🤖</div>
                <div>No connected servers</div>
                <div style="font-size: 12px; margin-top: 4px;">
                    Invite the bot to your server to get started
                </div>
            </div>
        `;
    return;
  }

  guildList.innerHTML = connectedGuilds
    .map(
      (guild) => `
        <div class="guild-card" data-guild-id="${guild.id}">
            <div class="guild-info">
                <div class="guild-icon">${guild.name[0].toUpperCase()}</div>
                <div>
                    <strong>${guild.name}</strong>
                    <div style="color: var(--text-muted); font-size: 12px;">
                        ${guild.owner ? "Owner" : "Member"} • ID: ${guild.id}
                    </div>
                </div>
            </div>
            <div style="color: var(--discord-green); font-size: 12px;">
                ✅ Bot Connected
                ${guild.voice_channel ? `<br>🔊 ${guild.voice_channel}` : ""}
                ${guild.queue_length > 0 ? `<br>🎵 ${guild.queue_length} in queue` : ""}
            </div>
        </div>
    `,
    )
    .join("");
}

// Periodic guild refresh functionality
function startGuildRefresh() {
  if (guildRefreshInterval) {
    clearInterval(guildRefreshInterval);
  }

  guildRefreshInterval = setInterval(async () => {
    if (!currentToken || !currentUser) {
      stopGuildRefresh();
      return;
    }

    try {
      console.log("Refreshing guild list...");
      const result = await apiCall("GET", "/api/guilds");

      if (result.status === 200 && result.data.success) {
        // Update the guild list silently (without showing the response popup)
        displayUserGuilds(result.data.data);
      } else if (result.status === 401) {
        // Token expired, stop refresh and log out
        console.log("Token expired, logging out...");
        logout();
      }
    } catch (error) {
      console.error("Error refreshing guilds:", error);
      // Don't show error popups for background refresh failures
      // Just log them to console
    }
  }, GUILD_REFRESH_INTERVAL);

  console.log("Started periodic guild refresh (every 30 seconds)");
}

function stopGuildRefresh() {
  if (guildRefreshInterval) {
    clearInterval(guildRefreshInterval);
    guildRefreshInterval = null;
    console.log("Stopped periodic guild refresh");
  }
}

function openModal(type) {
  currentModalType = type;
  const modal = document.getElementById("api-modal");
  const title = document.getElementById("modal-title");
  const form = document.getElementById("modal-form");

  const configs = {
    getQueue: {
      title: "Get Queue",
      form: `<div class="form-group">
                     <label>Guild ID:</label>
                     <select id="guild-id" required>${getGuildOptions()}</select>
                   </div>`,
    },
    addQueue: {
      title: "Add to Queue",
      form: `
                <div class="form-group">
                  <label>Guild ID:</label>
                  <select id="guild-id" required>${getGuildOptions()}</select>
                </div>
                <div class="form-group">
                  <label>Song URL:</label>
                  <input type="url" id="song-url" placeholder="https://www.youtube.com/watch?v=..." required>
                </div>
                <div class="form-group">
                  <label>Voice Channel ID (optional):</label>
                  <input type="text" id="channel-id" placeholder="Voice channel ID">
                </div>
            `,
    },
    skipTrack: {
      title: "Skip Track",
      form: `<div class="form-group">
                     <label>Guild ID:</label>
                     <select id="guild-id" required>${getGuildOptions()}</select>
                   </div>`,
    },
    clearQueue: {
      title: "Clear Queue",
      form: `<div class="form-group">
                     <label>Guild ID:</label>
                     <select id="guild-id" required>${getGuildOptions()}</select>
                   </div>`,
    },
    playPause: {
      title: "Play/Pause",
      form: `<div class="form-group">
                     <label>Guild ID:</label>
                     <select id="guild-id" required>${getGuildOptions()}</select>
                   </div>`,
    },
    stopPlayback: {
      title: "Stop Playback",
      form: `<div class="form-group">
                     <label>Guild ID:</label>
                     <select id="guild-id" required>${getGuildOptions()}</select>
                   </div>`,
    },
    joinVoice: {
      title: "Join Voice Channel",
      form: `
                <div class="form-group">
                  <label>Guild ID:</label>
                  <select id="guild-id" required>${getAllGuildOptions()}</select>
                </div>
                <div class="form-group">
                  <label>Voice Channel ID:</label>
                  <input type="text" id="channel-id" placeholder="123456789012345678" required>
                  <small style="color: var(--text-muted); display: block; margin-top: 4px;">
                    Right-click a voice channel in Discord → Copy ID
                  </small>
                </div>
            `,
    },
    setVolume: {
      title: "Set Volume",
      form: `
                <div class="form-group">
                  <label>Guild ID:</label>
                  <select id="guild-id" required>${getGuildOptions()}</select>
                </div>
                <div class="form-group">
                  <label>Volume (0.0 - 1.0):</label>
                  <input type="number" id="volume" min="0" max="1" step="0.1" value="0.5" required>
                </div>
            `,
    },
    songInfo: {
      title: "Get Song Info",
      form: `<div class="form-group">
                     <label>Song URL:</label>
                     <input type="url" id="song-url" placeholder="https://www.youtube.com/watch?v=..." required>
                   </div>`,
    },
  };

  const config = configs[type];
  title.textContent = config.title;
  form.innerHTML = config.form;

  // Hide response section
  document.getElementById("response-section").classList.add("hidden");

  modal.classList.add("visible");
}

function getGuildOptions() {
  if (!currentUser) return '<option value="">No guilds available</option>';

  const guildCards = document.querySelectorAll("[data-guild-id]");
  if (guildCards.length === 0)
    return '<option value="">No guilds available</option>';

  let options = '<option value="">Select a guild...</option>';
  guildCards.forEach((card) => {
    const guildId = card.getAttribute("data-guild-id");
    const guildName = card.querySelector("strong").textContent;
    options += `<option value="${guildId}">${guildName}</option>`;
  });

  return options;
}

function getAllGuildOptions() {
  if (!currentUser || allUserGuilds.length === 0) {
    return '<option value="">No guilds available</option>';
  }

  let options = '<option value="">Select a guild...</option>';
  allUserGuilds.forEach((guild) => {
    const connectedText = guild.connected ? " (Bot Connected)" : "";
    options += `<option value="${guild.id}">${guild.name}${connectedText}</option>`;
  });

  return options;
}

function closeModal() {
  document.getElementById("api-modal").classList.remove("visible");
  currentModalType = null;
}

async function executeModalCall() {
  if (!currentModalType) return;

  const executeBtn = document.getElementById("execute-btn");
  const executeText = document.getElementById("execute-text");
  const executeLoading = document.getElementById("execute-loading");

  executeBtn.disabled = true;
  executeText.classList.add("hidden");
  executeLoading.classList.remove("hidden");

  try {
    let result;
    const guildIdElement = document.getElementById("guild-id");
    const guildId = guildIdElement ? guildIdElement.value : null;

    if (guildId && !guildId.trim()) {
      Swal.fire({
        icon: "error",
        title: "Guild Required",
        text: "Please select a guild",
        confirmButtonColor: "#5865f2",
      });
      return;
    }

    switch (currentModalType) {
      case "getQueue":
        result = await apiCall("GET", `/api/queue/${guildId}`);
        break;
      case "addQueue":
        const url = document.getElementById("song-url").value;
        const channelId = document.getElementById("channel-id").value;
        result = await apiCall("POST", `/api/queue/${guildId}/add`, {
          url,
          channel_id: channelId || null,
        });
        break;
      case "skipTrack":
        result = await apiCall("POST", `/api/queue/${guildId}/skip`);
        break;
      case "clearQueue":
        result = await apiCall("DELETE", `/api/queue/${guildId}`);
        break;
      case "playPause":
        result = await apiCall("POST", `/api/control/${guildId}/play`);
        break;
      case "stopPlayback":
        result = await apiCall("POST", `/api/control/${guildId}/stop`);
        break;
      case "joinVoice":
        const voiceChannelId = document.getElementById("channel-id").value;
        result = await apiCall("POST", `/api/control/${guildId}/join`, {
          channel_id: voiceChannelId,
        });
        break;
      case "setVolume":
        const volume = parseFloat(document.getElementById("volume").value);
        result = await apiCall("PUT", `/api/control/${guildId}/volume`, {
          volume,
        });
        break;
      case "songInfo":
        const songUrl = document.getElementById("song-url").value;
        result = await apiCall(
          "GET",
          `/api/song/info?url=${encodeURIComponent(songUrl)}`,
        );
        break;
    }

    showModalResponse(result.status, result.data);
  } catch (error) {
    showModalResponse(500, { error: error.message });
  } finally {
    executeBtn.disabled = false;
    executeText.classList.remove("hidden");
    executeLoading.classList.add("hidden");
  }
}

function showModalResponse(status, data) {
  const responseSection = document.getElementById("response-section");
  const statusCode = document.getElementById("status-code");
  const responseBody = document.getElementById("response-body");

  statusCode.textContent = status;
  statusCode.className = `status-code status-${Math.floor(status / 100) * 100}`;
  responseBody.textContent = JSON.stringify(data, null, 2);

  responseSection.classList.remove("hidden");
}

function showResponse(status, data) {
  // Determine icon and title based on status
  let icon = "info";
  let title = "API Response";

  if (status >= 200 && status < 300) {
    icon = "success";
    title = "Success";
  } else if (status >= 400 && status < 500) {
    icon = "warning";
    title = "Client Error";
  } else if (status >= 500) {
    icon = "error";
    title = "Server Error";
  }

  Swal.fire({
    icon: icon,
    title: `${title} (${status})`,
    html: `<pre style="text-align: left; font-size: 12px; max-height: 300px; overflow-y: auto; background: #2f3136; padding: 15px; border-radius: 6px; color: #dcddde;">${JSON.stringify(data, null, 2)}</pre>`,
    width: "600px",
    confirmButtonColor: "#5865f2",
    customClass: {
      popup: "swal-dark-theme",
    },
  });
}
