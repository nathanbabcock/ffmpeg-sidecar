exit 1 # snippets only, not meant to be run together

# testsrc
ffmpeg \
  -v level+info \
  -f lavfi \
  -i testsrc \
  -y test.mp4

# pix_fmts
ffmpeg -hide_banner -pix_fmts
