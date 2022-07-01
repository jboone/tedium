#!/usr/bin/env python3

import time

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

ep_out = usb.util.find_descriptor(
    intf,
    custom_match=lambda e:
        usb.util.endpoint_direction(e.bEndpointAddress) == usb.util.ENDPOINT_OUT \
        and usb.util.endpoint_address(e.bEndpointAddress) == ENDPOINT
)

ep_in = usb.util.find_descriptor(
    intf,
    custom_match=lambda e:
        usb.util.endpoint_direction(e.bEndpointAddress) == usb.util.ENDPOINT_IN \
        and usb.util.endpoint_address(e.bEndpointAddress) == ENDPOINT
)

command_count = 0

while True:
    try:
        ep_out.write(bytes([0x00, 0xfe, 0x01]))
    except usb.core.USBError as e:
        print(f"OUT: {e}")

    try:
        result = ep_in.read(Descriptors.SOC_OUT_BYTES_MAX)
        if len(result) != 1:
            print(result)
        else:
            command_count += 1
            if command_count % 10000 == 0:
                print(f"{command_count}")
    except usb.core.USBError as e:
        print(f"IN: {e}")

