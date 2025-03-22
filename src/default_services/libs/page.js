let PAGEINDEX = 0;

function prefix_style(classname, style) {
    let lines = style.split('\n');
    let out = [];
    for (let i = 0;i < lines.length; i++) {
        if (lines[i] && /\.|#|\*|&|[a-z]|[A-Z]/.test(lines[i][0])) {
            out.push("." + classname + " " + lines[i]);
        } else if (lines[i]) {
            out.push(lines[i]);
        }
    }
    return out.join('\n');
}

class Page {
    constructor(name, page) {
        this.name = name || "";
        this.index = PAGEINDEX;
        PAGEINDEX += 1;
        this.identifier = "page-" + this.name + this.index;

        this.page = document.createElement("div");
        this.page.classList.add(this.identifier);
        this.page.innerHTML = page.content;
        
        this.style = document.createElement("style");
        this.style.innerHTML = prefix_style(this.identifier, page.style);

        this.script_content = page.script;
        this.script = (page) => {
            eval(this.script_content);
        };
    }
    append_to(element) {
        element.append(this.page);
        this.script(this);
    }
    create_style(style) {
        let style_element = document.createElement("style");
        style_element.innerHTML = prefix_style(this.identifier, style);
        return style_element;
    }
    apply_style(style) {
        if (style) {
            head.append(this.create_style(style));
        } else {
            head.append(this.style);
        }
    }
    get_base64(path, callback) {
        zeitop.request("page", this.name + "/" + path, callback, "");
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
        return this.page.querySelector("#" + id);
    }
    createElement(element) {
        console.log(element);
        let e = document.createElement(element);
        e.classList.add(this.identifier);
        return e;
    }
}

function request_page(name, callback) {
    zeitop.request("page", name, (page) => {
        callback(new Page(name, JSON.parse(page)));
    }, "");
}
