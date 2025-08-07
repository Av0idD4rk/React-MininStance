# Use the official slim Python image
FROM python:3.10-slim

# Create & switch to app dir
WORKDIR /app

# Copy our tiny server
COPY server.py .

# Expose the port our server listens on
EXPOSE 3000

# Run it
CMD ["python", "server.py"]
