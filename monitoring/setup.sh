#!/bin/bash
# Amber Monitoring Setup Script
# This script sets up a complete monitoring stack for Amber runtime

set -e

echo "🚀 Amber Monitoring Stack Setup"
echo "================================"
echo ""

# Check if Docker is installed
if ! command -v docker &> /dev/null; then
    echo "❌ Docker is not installed. Please install Docker first."
    exit 1
fi

# Check if Docker Compose is installed
if ! command -v docker-compose &> /dev/null; then
    echo "❌ Docker Compose is not installed. Please install Docker Compose first."
    exit 1
fi

# Create necessary directories
echo "📁 Creating monitoring directories..."
mkdir -p prometheus grafana/dashboards grafana/datasources

# Start the monitoring stack
echo "🔧 Starting monitoring stack..."
docker-compose up -d

# Wait for services to be ready
echo "⏳ Waiting for services to start..."
sleep 10

# Check service status
echo "📊 Checking service status..."
docker-compose ps

# Import Grafana dashboard
echo "📈 Importing Grafana dashboard..."
sleep 5

# Try to import the dashboard
DASHBOARD_FILE="grafana-dashboard.json"
if [ -f "$DASHBOARD_FILE" ]; then
    echo "✅ Dashboard file found: $DASHBOARD_FILE"
    # Note: Dashboard import would normally be done via Grafana API
    # For now, users can import manually via Grafana UI
else
    echo "⚠️  Dashboard file not found: $DASHBOARD_FILE"
fi

echo ""
echo "🎉 Monitoring stack setup complete!"
echo ""
echo "Access URLs:"
echo "  📊 Grafana:    http://localhost:3000 (admin/admin123)"
echo "  📈 Prometheus: http://localhost:9090"
echo "  🚨 AlertManager: http://localhost:9093"
echo "  💻 Node Exporter: http://localhost:9100"
echo ""
echo "Next steps:"
echo "1. Configure Amber to expose metrics on port 3000"
echo "2. Import the Amber dashboard in Grafana (if not auto-imported)"
echo "3. Configure alert destinations in AlertManager"
echo "4. Run your Amber workloads and monitor performance"
echo ""
echo "To stop the monitoring stack:"
echo "  docker-compose down"
echo ""
echo "To view logs:"
echo "  docker-compose logs -f [service_name]"
