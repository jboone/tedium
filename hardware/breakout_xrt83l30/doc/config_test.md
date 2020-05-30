# Configuration for Initial Test

3.3V power applied at ICT# 3.3V and ground jumper pins.

MCLKT1 receives 1.544MHz @ 3.3V from a signal generator

Unspecified pins are not connected.

### J3

RESET# = 1
TERSEL[1:0] = 2b00
RXTSEL = 1
TXTSEL = 1
JABW = 0
JASEL[1:0] = 2b00
GAUGE = X
RXMUTE = 0
RXRES[1:0] = 2b00
RCLKE = 0
TXTEST[2:0] = 3b000
TCLKE = 0
TXON = 1

### J6

INSBPV = 0
NLCDE[1:0] = 2b00
LOOP[1:0] = 2b00 (RX channel audio passes, but connected Adit interface will not be "up") or 2b10 (remote loopback, however hook doesn't work, audio mostly isn't passed)
SD/DR# = 1

TRATIO = 1
EQC0/INT# = 0
EQC1/CS# = 0
EQC2/SCLK = 0
EQC3/SDO = 1
EQC4/SDI = 0
HW/HOST# = 1
CLKSEL[2:0] = 3b001

### J1

TNEG_CODES = 0 (pulled down, don't need to connect)

### J2

RCLK outputs recovered receive clock
RPOS_RDATA outputs received data after B8ZS decoding

# Configuration 2020/04/01 (No Joke!)

## Datasheet Notes

Jitter Attenuator: "NOTE: If the LIU is used in a loop timing system, the jitter attenuator should be enabled in the receive path."

T1 book recommends running in "loop" mode, using RX as source of timing, and repeating that on the TX side.
In multi-span architectures, one span RX would be chosen as the source of timing, and hopefully the other spans would be on the same clock already?
In a peering-to-ShadyTel situation, we'd have a span to them act as our clock source, and we'd use that for spans to Adits, etc.?

FPGA sends 8kHz clock to MCLKE1 input on LIU

RESET#		= 1			// Normal operation
TERSEL[1:0]	= 2'b00		// Internal termination impedance: 100 Ohms
RXTSEL		= 1			// RX termination: internal			
TXTSEL		= 1			// TX termination: internal
JABW		= 0			// Jitter attenuator bandwidth: in T1, this setting doesn't matter, bandwidth is always 3Hz.
JASEL[1:0]	= 2'b11		// Jitter attenuator selection: attenuate jitter on T1 receiver, 3dB, 64-bit FIFO depth.
GAUGE       = _
RXMUTE		= 0
RXRES[1:0]	= 2'b00		// Receive external resistor: No external fixed resistor
RCLKE		= 0
TXTEST[2:0]	= 3'b000
TCLKE		= 0
TXON		= 1

INSBPV		= 0			// Insert bipolar violation: disabled
NLCDE[1:0]	= 2'b00		// Network loop code detection: disabled
LOOP[1:0]   = 2'b00		// Loopback: Normal mode
SR/DR#      = 1         // Single-rail data format
TRATIO      = 1			// 1:2 transmitter transformer ratio. (Ignored if using internal termination mode.)
EQC[4:0]    = 5'b01000	// E1/T1 mode & Receive Sensitivity: T1 Short Haul / 15dB, Transmit LBO: 0-133 ft. / 0.6dB, Cable: 100 Ohm / TP, Coding: B8ZS
CLKSEL[2:0] = 3'b010	// 8kHz input to MCLKE1, clkout depends on MCLKRATE (set by EQC[4:0] in strapping pin mode)
