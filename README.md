# Reddsaver

### Instructions


Running with docker: 
```
mkdir -pv data/
docker build -t reddsaver:v0.1.0 .
docker run --rm \
    --volume="$PWD/data:/app/data" \
    --volume="$PWD/dkr.env:/app/.env" \
    reddsaver:v0.1.0
```

