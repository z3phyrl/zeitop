let z = zeitop;

const hour = document.getElementById("hour");
const min = document.getElementById("min");

const cpu_bar = document.getElementById("cpu-bar");
const cpu_percent = document.getElementById("cpu-percent");
const mem_bar = document.getElementById("mem-bar");
const mem_val = document.getElementById("mem-val");
let uptime = 0;
let total_mem = null;

const music_title = document.getElementById("title");
const music_artists = document.getElementById("artists");
const music_bar = document.getElementById("music-bar");
const music_elapsed_span = document.getElementById("music-elapsed");
const music_duration_span = document.getElementById("music-duration");
const music_prev = document.getElementById("music-prev");
const music_play_pause = document.getElementById("music-play-pause");
const music_next = document.getElementById("music-next");
let music_state_playing = false;
let music_duration = null;
let music_elapsed = null;

function set_time() {
    let now = new Date();
    hour.innerText = ("" + now.getHours()).padStart(2, "0");
    min.innerText = ("" + now.getMinutes()).padStart(2, "0");
}

function set_uptime() {
    document.getElementById("uptime").innerHTML = ("" + Math.floor(uptime / 3600)).padStart(2, "0") + ":" + ("" + (Math.floor(uptime / 60) % 60)).padStart(2, "0") + ":" + ("" + (uptime % 60)).padStart(2, "0");
}

function update_bar(bar, percent) {
    for (let i = 0; i < bar.children.length; i++) {
        if (i <= (bar.children.length * percent)) {
            bar.children[i].style.backgroundColor = "#FFFFFF";
        } else {
            bar.children[i].style.backgroundColor = "#666666";
        }
    }
}

function avg_cpu() {
    z.request("sysinfo", "cpu", (cpu) => {
        let cpus = JSON.parse(cpu);
        let sum = 0;
        let count = 0;
        for (var key in cpus) {
            sum += cpus[key];
            count += 1;
        }
        let percent = Math.floor(sum / count);
        cpu_percent.innerText = percent + "%";
        update_bar(cpu_bar, percent / 100);
    }, "poll_info");
}

function mem() {
    z.request("sysinfo", "used_mem", (used_mem) => {
        let gb = ((parseInt(used_mem) / 1024) / 1024) / 1024;
        mem_val.innerText = (Math.round(gb * 100) / 100).toFixed(2) + "GB";
        update_bar(mem_bar, (used_mem / total_mem));
    }, "poll_info");
}

function bar_init(bar, width) {
    for (let k = 0; k < (bar.offsetWidth / (width)); k++) {
        bar.appendChild(document.createElement("div"));
    }
}

function music_set(title, artists) {
    music_title.innerText = title;
    for (let artist of artists) {
        music_artists.innerText += artist;
    }
}

function update_music() {
    update_bar(music_bar, music_elapsed / music_duration);
    music_elapsed_span.innerText = music_elapsed;
}

function update_music_state() {
    z.request("mpd", "status", (mpd_status) => {
        mpd_status = JSON.parse(mpd_status);
        if (mpd_status.state == "Playing") {
            music_state_playing = true;
        } else {
            music_state_playing = false;
        }
        music_duration = mpd_status.duration.secs;
        music_elapsed = mpd_status.elapsed.secs;
        music_duration_span.innerText = music_duration;
        music_elapsed_span.innerText = music_elapsed;
        music_play_pause.innerText = music_state_playing ? "||" : "|>";
        dbg(mpd_status);
    } ,"music");
}

z.request("page", "default/fonts/mononoki.ttf", (mononoki) => {
    load_b64_ttf("Mononoki", mononoki);
}, "init");
z.request("sysinfo", "user", (user) => {
    document.getElementById("user").innerHTML = user;
}, "init");
z.request("sysinfo", "host", (host) => {
    document.getElementById("host").innerHTML = host;
}, "init");
z.request("sysinfo", "uptime", (up_time) => {
    uptime += parseInt(up_time);
    set_uptime();
}, "init");
z.request("sysinfo", "total_mem", (mem) => {
    total_mem = parseInt(mem);
}, "init");
z.request("mpd", "currentsong", (currentsong) => {
    currentsong = JSON.parse(currentsong);
    music_set(currentsong.title, currentsong.artists)
}, "init");
z.request("mpd", "status", (mpd_status) => {
    update_music_state();
    update_music();
}, "init");
bar_init(cpu_bar, 6);
bar_init(mem_bar, 6);
bar_init(music_bar, 6);

setInterval(() => {
    set_time()
    uptime += 1;
    set_uptime()
    avg_cpu();
    mem();
    if (music_state_playing) {
        if (music_elapsed < music_duration) {
            music_elapsed += 1;
        }
    }
    update_music();
}, 1000);

set_time(); 

z.subscribe("mpd-events", (subsystem) => {
    update_music_state();
    z.request("mpd", "currentsong", (currentsong) => {
        currentsong = JSON.parse(currentsong);
        update_music(currentsong.title, currentsong.artists)
    }, "music")
});

music_prev.addEventListener("click", () => {
    z.request("mpd", "prev", (ok) => {
    }, "music-control")
});
music_play_pause.addEventListener("click", () => {
    z.request("mpd", music_state_playing ? "pause" : "play", (ok) => {
    }, "music-control")
});
music_next.addEventListener("click", () => {
    z.request("mpd", "next", (ok) => {
    }, "music-control")
});
