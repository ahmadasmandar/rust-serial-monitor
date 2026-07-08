#include <Arduino.h>

void setup() {
  Serial.begin(921600);
  while (!Serial) {
    ; // Wait for hardware serial USB port to connect
  }
}

uint32_t frameCounter = 0;

void loop() {
  uint8_t frame[100];
  
  // Header (2 bytes)
  frame[0] = 0xAA;
  frame[1] = 0x55;
  
  // Safe Counter: 4 bytes formatted as ASCII decimal digits to avoid binary 0x0A/0x0D splits
  char counterBuf[5];
  snprintf(counterBuf, sizeof(counterBuf), "%04lu", (unsigned long)(frameCounter % 10000));
  memcpy(&frame[2], counterBuf, 4);
  
  // Safe Predictable Payload (92 bytes, values 32..121)
  for (int i = 6; i < 98; i++) {
    frame[i] = (uint8_t)(32 + (i - 6) % 90);
  }
  
  // XOR Checksum calculation (bytes 0 to 97)
  uint8_t checksum = 0;
  for (int i = 0; i < 98; i++) {
    checksum ^= frame[i];
  }
  
  // Dynamically adjust checksum to prevent it from being 0x0A (LF) or 0x0D (CR)
  if (checksum == 0x0A || checksum == 0x0D) {
    frame[97] ^= 0x20; // Modify last payload byte to change checksum
    checksum ^= 0x20;  // Recalculate checksum adjustment
  }
  frame[98] = checksum;
  
  // Footer / Delimiter (1 byte)
  frame[99] = '\n';
  
  // Transmit the 100-byte binary frame
  Serial.write(frame, 100);
  
  frameCounter++;
  delay(1); // 1 kHz frequency
}


