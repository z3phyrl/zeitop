# Zeitop

Use android phone as a desktop clock and more.

## **WARNING**
- this project is ***unfinished****(barely working)*.
- need some setup for everything to work.

## Features
- Make your own page in HTML CSS and Javascript.
- Add your own Services.
- That's it.

## Roadmap
- [ ] add windows support
- [ ] deal with config stuff
- [ ] rewrite default services that I quickly spin up for the demo
- [ ] add some more useful default services
- [ ] add a wrapper page to handle switching page from client
- [ ] implement upgrading to broadcast service
- [ ] make a simple install script

## How to run on Linux
1. Compile and Install [client](https://github.com/z3phyrl/zeitop-client) in /usr/share/zeitop/ and rename it base.apk
2. Compile and install clean-up script
    ```sh
    cd clean-up
    bash build.sh
    cp build/cleaner.jar /usr/share/zeitop/
    ```
3. Run
    - Either move example page to ```$HOME/.config/zeitop/``` then run with ```cargo run```
    - Or run with command ```XDG_CONFIG_HOME=./examples/ cargo run```

## Windows
Currently windows is not supported but it will be in the future.
