#!/usr/bin/bash

adb push build/cleaner.jar /data/local/tmp/
adb shell CLASSPATH=/data/local/tmp/cleaner.jar nohup app_process / com.z3phyrl.MainKt
