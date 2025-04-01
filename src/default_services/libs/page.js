"use strict";

let PAGEINDEX = 0;

function prefix_style(classname, style) {
    let lines = style.split('\n');
    let out = [];
    for (let i = 0;i < lines.length; i++) {
        if (lines[i].startsWith(".root")) {
            out.push(lines[i].slice(5));
        } else if (lines[i].startsWith(".this")) {
            out.push('div[class="' + classname + '"]' + lines[i].slice(5));
        } else if (lines[i] && /\.|#|\*|[a-z]|[A-Z]/.test(lines[i][0])) {
            out.push("." + classname + " " + lines[i]);
        } else if (lines[i]) {
            out.push(lines[i]);
        }
    }
    return out.join('\n');
}

class Page {
    constructor(name, page) {
        this.zeitop = zeitop;
        this.name = name || "";
        this.index = PAGEINDEX;
        PAGEINDEX += 1;
        this.identifier = "page-" + this.name + this.index;

        this.element = document.createElement("div");
        this.element.classList.add(this.identifier);
        this.element.innerHTML = page.content;
        Object.assign(this.element, {page: this});
        
        this.style = null;
        if (page.style) {
            this.style = document.createElement("style");
            this.style.innerHTML = prefix_style(this.identifier, page.style);
        }
        this.libs = "";
        this.script_content = page.script;
    }
    new_connection(port) {
        this.zeitop = new Zeitop(device.serial, port || ZEITOP_PORT || 6969, () => {});
    }
    request(service, request, callback, tag, timeout) {
        this.zeitop.request(service, request, callback, this.identifier + "-" + this.zeitop.auto_num(service, tag), timeout);
    }
    subscribe(service, callback, tag) {
        this.zeitop.subscribe(service, callback, this.identifier + "-" + this.zeitop.auto_num(service, tag));
    }
    append_to(element) {
        element.append(this.element);
    }
    prepend_to(element) {
        element.prepend(this.element);
    }
    append_lib(name, str) {
        this.libs += "\n" + "// " + name + ".js\n" + str + "\n";
    }
    load_lib(name) {
        this.get("libs/" + name + ".js", (lib) => {
            if (lib) {
                this.append_lib(name, lib);
            } else if (lib == null) {
                this.request("lib", name, (lib) => {
                    if (lib) {
                        this.append_lib(name, lib);
                    }
                }, "load_lib-" + name + "-####");
            }
        });
    }
    load_page_libs(on_finish) {
        this.query("libs/", (entries) => {
            if (!entries) {
                on_finish();
                return;
            }
            for (let i = 0;i < entries.length; i++) {
                let entry = entries[i];
                if (entry.file_type == "File" && entry.name.endsWith(".js")) {
                    this.get("libs/" + entry.name, (lib) => {
                        if (lib) {
                            this.append_lib(entry.name, lib);
                        }
                        if (i + 1 == entries.length) {
                            on_finish();
                        }
                    });
                }
            }
        });
    }
    init() {
        if (!this.initialized) {
            this.initialized = true;
            let page = this;
            this.load_page_libs(() => {
                eval('"use strict";\n' + this.libs + this.script_content);
            });
        } else {
            log(this.identifier + " is already Initialized");
        }
    }
    create_style(style) {
        let style_element = document.createElement("style");
        style_element.innerHTML = prefix_style(this.identifier, style);
        return style_element;
    }
    apply_style(style) {
        if (style) {
            head.append(this.create_style(style));
        } else if (this.style) {
            head.append(this.style);
        }
    }
    query(path, callback) {
        this.request("page", this.name + "/" + path + "?", (res) => {
            callback(JSON.parse(res));
        }, "query-####");
    }
    get_base64(path, callback) {
        this.request("page", this.name + "/" + path, callback, "get_base64-####");
    }
    get(path, callback) {
        this.get_base64(path, (base64) => {
            callback(window.atob(base64));
        });
    }
    load_base64_ttf(font_family, b64) {
        head.append(this.create_style("@font-face {font-family: '" + font_family + "';src: url(data:font/truetype;charset=utf-8;base64," + b64 + ") format('truetype')}"));
    }
    load_ttf(font_family ,path) {
        this.get_base64(path, (base64) => {this.load_base64_ttf(font_family, base64)});
    }

    getElementById(id) {
        return this.element.querySelector("#" + id);
    }
    createElement(element) {
        return document.createElement(element);
    }
}

function request_page(name, callback) {
    zeitop.request("page", name, (page) => {
        callback(new Page(name, JSON.parse(page)));
    }, "");
}
