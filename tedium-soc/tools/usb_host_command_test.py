#!/usr/bin/env python3

import sys

import usb.core

from tedium.gateware.usb.descriptors_vendor import Descriptors

INTERFACE = Descriptors.InterfaceNumber.FramerControl
ENDPOINT = Descriptors.EndpointNumber.FramerControl
ALTERNATE_SETTING = 0

dev = usb.core.find(
    idVendor=Descriptors.VENDOR_ID,
    idProduct=Descriptors.PRODUCT_ID
)

cfg = dev.get_active_configuration()
intf = cfg[(INTERFACE,ALTERNATE_SETTING)]
print(intf)
dev.set_interface_altsetting(
    interface=INTERFACE,
    alternate_setting=ALTERNATE_SETTING
)

class HostCommand:
    def __init__(self, intf):
        self._ep_out = usb.util.find_descriptor(
            intf,
            custom_match=lambda e:
                usb.util.endpoint_direction(e.bEndpointAddress) == usb.util.ENDPOINT_OUT \
                and usb.util.endpoint_address(e.bEndpointAddress) == ENDPOINT
        )
        self._ep_in = usb.util.find_descriptor(
            intf,
            custom_match=lambda e:
                usb.util.endpoint_direction(e.bEndpointAddress) == usb.util.ENDPOINT_IN \
                and usb.util.endpoint_address(e.bEndpointAddress) == ENDPOINT
        )
        self._command_count = 0

    def _send_command(self, b: bytes):
        try:
            self._command_count += 1
            self._ep_out.write(b)
        except usb.core.USBError as e:
            if e.errno == 19:
                print("OUT: disconnected")
                sys.exit(-1)
            elif e.errno == 110:
                print(f"OUT: {e}")
            else:
                raise e

    def _await_response(self) -> bytes:
        try:
            return self._ep_in.read(Descriptors.SOC_OUT_BYTES_MAX)
        except usb.core.USBError as e:
            if e.errno == 19:
                print("IN: disconnected")
                sys.exit(-1)
            elif e.errno == 110:
                print(f"IN: {e}")
            else:
                raise e

    def _execute(self, b: bytes) -> bytes:
        self._send_command(b)
        return self._await_response()

    def register_read(self, address: int) -> int:
        r = self._execute([0x00, address & 0xff, address >> 8])
        return r[0]

    def register_write(self, address: int, value: int):
        self._execute([0x01, address & 0xff, address >> 8, value])

def test_fast_writes(command: HostCommand):
    while True:
        command.register_read(0x01fe)

        if command._command_count % 10000 == 0:
            print(f"{command._command_count}")

def test_ring_first_channel(command: HostCommand):
    import time
    while True:
        # Loop start
        IDLE = 0x50
        OUTGOING = 0xf0
        INCOMING_FROM_FXO = 0x00
        INCOMING_TO_FXO = 0xa0

        print("off-hook")
        command.register_write(0x0340, INCOMING_FROM_FXO | 0x05)    # IDLE
        time.sleep(2.0)

        print("on-hook")
        command.register_write(0x0340, IDLE | 0x05)
        time.sleep(4.0)

command = HostCommand(intf)

# test_fast_writes(command)
test_ring_first_channel(command)
