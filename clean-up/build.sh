#!/usr/bin/bash

kotlinc -cp libs/Java-WebSocket-1.6.0.jar -include-runtime Main.kt -d build/build.jar
d8 build/build.jar libs/Java-WebSocket-1.6.0.jar libs/slf4j-api-2.0.5.jar --output build/cleaner.jar

