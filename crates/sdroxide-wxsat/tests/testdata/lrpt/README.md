# Meteor-M2 LRPT recording (3 s)

`meteor_lrpt_137900_3s.wav` is 3 seconds cut from a pass recorded by F4NRM and
given to this project for its tests (`LRPT_16-29-02_137900kHz.wav`, 105 s, from
second 40 to second 43).

- Meteor-M2 LRPT downlink, 137.900 MHz at the centre of the recording.
- Complex baseband, 2 channels (I, Q), 16 bit, 187.5 kS/s.
- The carrier is about 1.3 kHz above the centre here, and moves by about -22 Hz
  every second (Doppler).
- QPSK at 72 ksym/s, so 2.604 samples per symbol. Es/N0 is about 8 dB.
- The recorder's clock is 7.5 ppm away from the satellite's: about 14 samples
  in 10 s. A symbol timing loop that does not follow that drift loses the signal.
- It holds about 26 frames of 1024 bytes (sync `1ACFFC1D`).

Used by the LRPT decoder's tests in `sdroxide-wxsat`.
