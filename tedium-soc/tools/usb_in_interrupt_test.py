#!/usr/bin/env python3

from io import BufferedReader, BytesIO
import sys

import array
from enum import IntEnum
import struct

import usb.core

from tedium.gateware.usb.descriptors_vendor import Descriptors

INTERFACE_REPORT = Descriptors.InterfaceNumber.Interrupt
ENDPOINT_REPORT = Descriptors.EndpointNumber.Interrupt
ALTERNATE_SETTING = 0 #Descriptors.AlternateSetting.Active

dev = usb.core.find(idVendor=Descriptors.VENDOR_ID, idProduct=Descriptors.PRODUCT_ID)
cfg = dev.get_active_configuration()
intf = cfg[(INTERFACE_REPORT,ALTERNATE_SETTING)]
print(intf)
# intf.set_altsetting()
dev.set_interface_altsetting(interface=INTERFACE_REPORT, alternate_setting=ALTERNATE_SETTING)

ep_report_in = usb.util.find_descriptor(
    intf,
    custom_match=lambda e:
        usb.util.endpoint_direction(e.bEndpointAddress) == usb.util.ENDPOINT_IN \
        and usb.util.endpoint_address(e.bEndpointAddress) == ENDPOINT_REPORT
)

def format_bytes_hex(d):
    result = []
    for v in d:
        result.append(f"{v:02x}")
    return ' '.join(result)

while True:
    try:
        report = ep_report_in.read(Descriptors.INTERRUPT_BYTES_MAX)
        # report = bytes(report)
        if len(report) > 0:
            r = BytesIO(report)
            channel, bisr = r.read(2)

            if bisr & 0x40: # LBCODE
                rlcisrs = []
                for i in range(8):
                    rlcisr_x = r.read(1)[0]
                    rlcisrs.append(rlcisr_x)
                rlcisrs = ' '.join(f"{v:02x}" for v in rlcisrs)
                print(f"{channel} RLCISRx=[{rlcisrs}]")

            if bisr & 0x08: # HDLC
                for i in range(3):
                    dlsr = r.read(1)[0]
                    if dlsr & 0x08: # RxEOT
                        rdlbcr = r.read(1)[0]
                        rdlbc = rdlbcr & 0x7f
                        lapdbcr = r.read(rdlbc)
                        lapdbcr_s = format_bytes_hex(lapdbcr)
                        print(f"{channel} DLSR{i}={dlsr:02x} HDLC{i}=[{lapdbcr_s}]")
                    elif dlsr != 0:
                        print(f"{channel} DLSR{i}={dlsr:02x}")
            
            if bisr & 0x04: # SLIP
                sbisr = r.read(1)[0]
                print(f"{channel} SBISR={sbisr:02x}")

            if bisr & 0x02: # ALARM
                aeisr, exzsr, ciasr = r.read(3)
                print(f"{channel} AEISR={aeisr:02x} EXZSR={exzsr:02x} CIASR={ciasr:02x}")

            if bisr & 0x01: # T1FRAME
                fisr = r.read(1)[0]
                if fisr & 0x20: # SIG
                    rsar = r.read(12)
                    rasr_s = format_bytes_hex(rsar)
                    print(f"{channel} FISR={fisr:02x} RASR=[{rasr_s}]")
                else:
                    print(f"{channel} FISR={fisr:02x}")
                    
        elif len(report) == 0:
            pass
        else:
            print('read(): {report}')
    except usb.core.USBError as e:
        print(e)

