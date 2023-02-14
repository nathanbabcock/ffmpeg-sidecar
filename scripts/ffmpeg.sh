exit 1 # snippets only, not meant to be run together

# testsrc
ffmpeg \
  -v level+info \
  -f lavfi \
  -i testsrc \
  -y output/test.mp4

# to /dev/null
ffmpeg \
  -v level+info \
  -f lavfi \
  -i testsrc \
  -f rawvideo \
  -pix_fmt rgb24 \
  pipe > /dev/null
# to stdout: 'pipe', 'pipe:', 'pipe:1', '-'
# to stderr: 'pipe:2'

# pix_fmts
ffmpeg -hide_banner -pix_fmts
