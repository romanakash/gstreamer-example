mkdir -p samples/
yt-dlp https://www.youtube.com/watch?v=H0Yirlo6WSU --output ./samples/hammy -f "bestvideo[ext=mp4]+bestaudio[ext=m4a]/best[ext=mp4]/best"
