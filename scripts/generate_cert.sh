#!/bin/bash
# Génération d'un certificat auto-signé pour HTTPS

echo "🔐 Génération d'un certificat SSL auto-signé..."

mkdir -p certs

openssl req -x509 -newkey rsa:4096 -nodes \
    -keyout certs/key.pem \
    -out certs/cert.pem \
    -days 365 \
    -subj "/CN=phonecam.local" \
    -addext "subjectAltName=DNS:phonecam.local,IP:192.168.100.6,IP:127.0.0.1"

echo "✅ Certificat créé dans ./certs/"
echo ""
echo "⚠️  IMPORTANT : Sur ton smartphone, tu devras accepter le certificat non sécurisé"
echo "    lors de la première connexion (option 'Avancé' > 'Continuer quand même')"
