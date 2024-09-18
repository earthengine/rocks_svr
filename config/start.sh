#!/bin/sh

echo "Starting Nginx..."
# Start Nginx in the background
nginx &
echo "Nginx started!"

# Your other application command here
# For example:
# python your_app.py

# If you have another command, replace the echo with your command
echo "Nginx is running in the background. Starting your app..."
rocks_svr;

ps -A
